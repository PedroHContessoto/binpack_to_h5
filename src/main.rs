// binpack_to_h5: convert Stockfish binpack to HDF5 sparse_v1 (white-POV).
//
// Multi-threaded pipeline:
//   1 reader thread  - decompresses binpack via sfbinpack crate
//   N encoder threads - filter + encode HalfP features (parallelizable)
//   1 writer thread  - writes HDF5 datasets (sequential, HDF5 not thread-safe)
//
// FILTERS (match nnue-pytorch SparseBatchProvider with filtered=True default):
//   - skip captures (~12-20% of positions): eval depends on QS, search handles
//   - skip in-check (~5-10%): STM forced to respond, static eval irrelevant
//   Total filtered: ~25-35% of raw positions.
//   Without filter, models train on noisy tactical positions and underperform.
//   Reference: github.com/official-stockfish/nnue-pytorch
//
// Output format: HDF5 sparse_v1 (compatible with nnue-pytorch sparse loaders).
//   - train/val groups, each with: evals, has_eval, has_wdl, stm, piece_count,
//     wdl, offsets, w_flat (sparse HalfP feature indices)
//
// Usage:
//   binpack_to_h5 \
//     --binpack /path/to/input.binpack \
//     --output  /path/to/output.h5 \
//     [--workers N]            # default num_cpus, 8 min, 16 recommended
//     [--max-positions N]      # raw read count; output = ~70% after filter
//     [--val-fraction 0.005]   # fraction held out for validation split
//
// Expected performance:
//   reader:  ~5-10M entries/sec (binpack decompress is the bottleneck)
//   writer:  ~10-20M positions/sec (HDF5 chunked write)
//   total:   ~3-5M positions/sec end-to-end
//   E.g., 2.67B raw entries -> ~1.85B after filter in ~10-15 min on 16 cores

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use hdf5_metno::Dataset;
use hdf5_metno::File as H5File;
use hdf5_metno::Group;
use indicatif::{ProgressBar, ProgressStyle};
use sfbinpack::chess::piece::Piece;
use sfbinpack::chess::position::Position;
use sfbinpack::chess::r#move::{Move, MoveType};
use sfbinpack::CompressedTrainingDataEntryReader;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

// ============================================================================
// Constants — written as HDF5 attrs so consumers can validate the pipeline
// ============================================================================
const PIPELINE_VERSION: &str = "sf_binpack_halfp_v1";
const FORMAT_VERSION: &str = "sparse_v1";
const EVAL_PERSPECTIVE: &str = "white_pov_assumed";
// EVAL_SCALE: divisor applied to centipawn scores before storage.
//   stored_eval = clip(cp / EVAL_SCALE, ±TARGET_CLIP)  for normal evals
//   stored_eval = sign * (MATE_BASE - ply_decay)        for mate scores
// 400.0 is the Stockfish-baseline value. Lower (e.g. 300) gives finer
// resolution around equal positions but more saturation on decisive ones.
const EVAL_SCALE: f32 = 400.0;
const TARGET_CLIP: f32 = 6.5;
const MATE_BASE: f32 = 10.0;
const MATE_DECAY: f32 = 0.1;
const MATE_THRESHOLD_CP: i32 = 25000;
const MIN_DEPTH: u32 = 9;
const WDL_TEMPERATURE: f32 = 200.0;

const BATCH_SIZE: usize = 8192; // Entries per channel batch

#[derive(Parser, Debug)]
#[command(version, about = "Convert Stockfish binpack to HDF5 sparse_v1 (HalfP white-POV)")]
struct Args {
    /// Input .binpack path
    #[arg(long)]
    binpack: PathBuf,

    /// Output .h5 path (single file, will be overwritten)
    #[arg(long)]
    output: PathBuf,

    /// Encoder worker count (default = num_cpus, 16 recommended max)
    #[arg(long)]
    workers: Option<usize>,

    /// Cap on raw positions read from binpack (default: read all)
    #[arg(long)]
    max_positions: Option<u64>,

    /// Fraction held out for the val split (default 0.005 = 0.5%)
    #[arg(long, default_value_t = 0.005)]
    val_fraction: f64,

    /// Flush HDF5 every N accumulated positions (default 500k)
    #[arg(long, default_value_t = 500_000)]
    flush_every: u64,

    /// Channel buffer size, in batches of BATCH_SIZE entries each
    #[arg(long, default_value_t = 16)]
    channel_capacity: usize,
}

// ============================================================================
// FILTERING (Stockfish-style: captures + in-check)
// ============================================================================
// Matches the official nnue-pytorch SparseBatchProvider default (filtered=True).
// Skips ~25-35% of raw positions:
//   - captures: eval is in flux (depends on quiescence/recaptures) — search
//     handles tactics, the static eval shouldn't be trained on these
//   - in-check: side-to-move is forced to respond, static eval is meaningless
// Including these positions in training degrades model quality significantly.
// Reference: github.com/official-stockfish/nnue-pytorch
//   data_loader/cpp/training_data_loader.cpp - make_skip_predicate

#[inline(always)]
fn is_capture(pos: &Position, mv: &Move) -> bool {
    // Castle eh encoded como king-to-rook square (NAO eh captura real)
    if mv.mtype() == MoveType::Castle {
        return false;
    }
    // En passant captures the pawn behind the destination square (always a capture)
    if mv.mtype() == MoveType::EnPassant {
        return true;
    }
    // Normal capture: destination square holds any piece (must be enemy in legal moves)
    pos.piece_at(mv.to()) != Piece::NONE
}

#[inline(always)]
fn should_skip_entry(entry: &sfbinpack::TrainingDataEntry) -> bool {
    // Filter 1: skip captures (Stockfish filtered=True default, ~12-20% of positions)
    if is_capture(&entry.pos, &entry.mv) {
        return true;
    }
    // Filter 2: skip in-check positions (Stockfish filtered=True default, ~5-10%)
    if entry.pos.is_checked(entry.pos.side_to_move()) {
        return true;
    }
    false
}

// ============================================================================
// Encoding HalfP white-POV (768 features = 2 colors × 6 piece types × 64 squares)
// ============================================================================

#[inline(always)]
fn encode_halfp(pos: &sfbinpack::chess::position::Position, buf: &mut Vec<i16>) {
    buf.clear();
    for sq in 0..64u32 {
        let p = pos.piece_at(sfbinpack::chess::coords::Square::new(sq));
        if p == Piece::NONE {
            continue;
        }
        let kind = p.piece_type().ordinal() as i16;
        let color = p.color().ordinal() as i16;
        let idx = color * 384 + kind * 64 + sq as i16;
        buf.push(idx);
    }
}

#[inline(always)]
fn normalize_eval(score_cp: i32) -> f32 {
    let abs_cp = score_cp.unsigned_abs() as i32;
    let sign: f32 = if score_cp >= 0 { 1.0 } else { -1.0 };
    if abs_cp > MATE_THRESHOLD_CP {
        let ply_estimate = (((32000 - abs_cp) as f32 / 100.0).max(1.0).min(30.0)) as f32;
        sign * (MATE_BASE - ply_estimate * MATE_DECAY)
    } else {
        sign * (abs_cp as f32 / EVAL_SCALE).min(TARGET_CLIP)
    }
}

// ============================================================================
// Channel message types
// ============================================================================

/// Reader -> encoder workers
struct ReadBatch {
    entries: Vec<sfbinpack::TrainingDataEntry>,
    base_idx: u64,
}

/// One encoded position (worker -> writer)
struct EncodedPos {
    eval: f32,
    stm: u8,
    piece_count: u8,
    wdl: f32,
    features: Vec<i16>,
    is_val: bool,
}

/// Batch of encoded positions (worker -> writer)
struct EncodedBatch {
    positions: Vec<EncodedPos>,
}

// ============================================================================
// PositionBuffer + H5Group: in-memory accumulator + extensible HDF5 datasets
// ============================================================================

struct PositionBuffer {
    evals: Vec<f32>,
    has_eval: Vec<u8>,
    has_wdl: Vec<u8>,
    stm: Vec<u8>,
    piece_count: Vec<u8>,
    wdl: Vec<f32>,
    w_flat: Vec<i16>,
    offsets_relative: Vec<i64>,
}

impl PositionBuffer {
    fn new() -> Self {
        Self {
            evals: Vec::new(),
            has_eval: Vec::new(),
            has_wdl: Vec::new(),
            stm: Vec::new(),
            piece_count: Vec::new(),
            wdl: Vec::new(),
            w_flat: Vec::new(),
            offsets_relative: vec![0i64],
        }
    }

    fn len(&self) -> usize {
        self.evals.len()
    }

    fn add(&mut self, ev: f32, st: u8, pc: u8, wdl_val: f32, features: &[i16]) {
        self.evals.push(ev);
        self.has_eval.push(1);
        self.has_wdl.push(1);
        self.stm.push(st);
        self.piece_count.push(pc);
        self.wdl.push(wdl_val);
        self.w_flat.extend_from_slice(features);
        let last = *self.offsets_relative.last().unwrap();
        self.offsets_relative.push(last + features.len() as i64);
    }

    fn clear(&mut self) {
        self.evals.clear();
        self.has_eval.clear();
        self.has_wdl.clear();
        self.stm.clear();
        self.piece_count.clear();
        self.wdl.clear();
        self.w_flat.clear();
        self.offsets_relative.clear();
        self.offsets_relative.push(0);
    }
}

struct H5Group {
    n_written: usize,
    w_flat_offset: i64,
    evals: Dataset,
    has_eval: Dataset,
    has_wdl: Dataset,
    stm: Dataset,
    piece_count: Dataset,
    wdl: Dataset,
    offsets: Dataset,
    w_flat: Dataset,
}

impl H5Group {
    fn create(parent: &Group) -> Result<Self> {
        let chunk_pos = 65536usize;
        let chunk_w = 1usize << 20;

        let evals = parent
            .new_dataset::<f32>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("evals")?;
        let has_eval = parent
            .new_dataset::<u8>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("has_eval")?;
        let has_wdl = parent
            .new_dataset::<u8>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("has_wdl")?;
        let stm = parent
            .new_dataset::<u8>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("stm")?;
        let piece_count = parent
            .new_dataset::<u8>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("piece_count")?;
        let wdl = parent
            .new_dataset::<f32>()
            .shape((0..,))
            .chunk((chunk_pos,))
            .create("wdl")?;
        let offsets = parent
            .new_dataset::<i64>()
            .shape((1..,))
            .chunk((chunk_pos,))
            .create("offsets")?;
        let w_flat = parent
            .new_dataset::<i16>()
            .shape((0..,))
            .chunk((chunk_w,))
            .create("w_flat")?;

        Ok(Self {
            n_written: 0,
            w_flat_offset: 0,
            evals,
            has_eval,
            has_wdl,
            stm,
            piece_count,
            wdl,
            offsets,
            w_flat,
        })
    }

    fn flush(&mut self, buf: &PositionBuffer) -> Result<()> {
        if buf.len() == 0 {
            return Ok(());
        }
        let n = buf.len();

        self.evals.resize((self.n_written + n,))?;
        self.evals
            .write_slice(&buf.evals, self.n_written..self.n_written + n)?;

        self.has_eval.resize((self.n_written + n,))?;
        self.has_eval
            .write_slice(&buf.has_eval, self.n_written..self.n_written + n)?;

        self.has_wdl.resize((self.n_written + n,))?;
        self.has_wdl
            .write_slice(&buf.has_wdl, self.n_written..self.n_written + n)?;

        self.stm.resize((self.n_written + n,))?;
        self.stm
            .write_slice(&buf.stm, self.n_written..self.n_written + n)?;

        self.piece_count.resize((self.n_written + n,))?;
        self.piece_count
            .write_slice(&buf.piece_count, self.n_written..self.n_written + n)?;

        self.wdl.resize((self.n_written + n,))?;
        self.wdl
            .write_slice(&buf.wdl, self.n_written..self.n_written + n)?;

        let abs_offsets: Vec<i64> = buf
            .offsets_relative
            .iter()
            .skip(if self.n_written == 0 { 0 } else { 1 })
            .map(|&x| x + self.w_flat_offset)
            .collect();
        let offset_start = if self.n_written == 0 { 0 } else { self.n_written + 1 };
        let offset_end = offset_start + abs_offsets.len();
        self.offsets.resize((offset_end,))?;
        self.offsets
            .write_slice(&abs_offsets, offset_start..offset_end)?;

        let w_n = buf.w_flat.len();
        self.w_flat.resize((self.w_flat_offset as usize + w_n,))?;
        self.w_flat.write_slice(
            &buf.w_flat,
            self.w_flat_offset as usize..self.w_flat_offset as usize + w_n,
        )?;
        self.w_flat_offset += w_n as i64;

        self.n_written += n;
        Ok(())
    }
}

// ============================================================================
// Encoder worker — receives ReadBatch, encodes, sends EncodedBatch to writer
// ============================================================================

fn encoder_worker(
    rx_read: Receiver<ReadBatch>,
    tx_encoded: Sender<EncodedBatch>,
    val_fraction: f64,
    failed_counter: Arc<AtomicU64>,
    filtered_counter: Arc<AtomicU64>,
) -> Result<()> {
    let val_threshold = (val_fraction * 1000.0) as u64;
    let mut features: Vec<i16> = Vec::with_capacity(40);
    let mut local_failed: u64 = 0;
    let mut local_filtered: u64 = 0;

    while let Ok(item) = rx_read.recv() {
        let mut encoded_positions = Vec::with_capacity(item.entries.len());
        for (i, entry) in item.entries.iter().enumerate() {
            // Stockfish-style filter (captures + in-check)
            if should_skip_entry(entry) {
                local_filtered += 1;
                continue;
            }
            encode_halfp(&entry.pos, &mut features);
            if features.is_empty() {
                local_failed += 1;
                continue;
            }
            let pc = features.len() as u8;
            let stm_val = entry.pos.side_to_move().ordinal();
            // STM-relative -> white-POV
            let stm_is_white = stm_val == 0;
            let raw_score = entry.score as i32;
            let raw_result = entry.result as f32;
            let score_white_pov = if stm_is_white { raw_score } else { -raw_score };
            let result_white_pov = if stm_is_white { raw_result } else { -raw_result };
            let ev = normalize_eval(score_white_pov);

            // Deterministic train/val split based on global entry index
            let global_idx = item.base_idx + i as u64;
            let is_val = (global_idx % 1000) < val_threshold;

            encoded_positions.push(EncodedPos {
                eval: ev,
                stm: stm_val,
                piece_count: pc,
                wdl: result_white_pov,
                features: features.clone(),
                is_val,
            });
        }
        // Send the encoded batch to the writer
        if !encoded_positions.is_empty() {
            if tx_encoded.send(EncodedBatch { positions: encoded_positions }).is_err() {
                break; // Writer fechou
            }
        }
    }

    failed_counter.fetch_add(local_failed, Ordering::Relaxed);
    filtered_counter.fetch_add(local_filtered, Ordering::Relaxed);
    Ok(())
}

// ============================================================================
// Writer thread — single thread, owns H5File, all writes sequential
// ============================================================================

fn writer_thread(
    output_path: PathBuf,
    binpack_path: PathBuf,
    rx: Receiver<EncodedBatch>,
    flush_every: u64,
    counter: Arc<AtomicU64>,
    train_counter: Arc<AtomicU64>,
    val_counter: Arc<AtomicU64>,
    val_fraction: f64,
) -> Result<()> {
    let h5 = H5File::create(&output_path)
        .with_context(|| format!("failed to create HDF5: {:?}", output_path))?;
    let train_grp = h5.create_group("train")?;
    let val_grp = h5.create_group("val")?;
    let mut train = H5Group::create(&train_grp)?;
    let mut val = H5Group::create(&val_grp)?;

    let mut buf_train = PositionBuffer::new();
    let mut buf_val = PositionBuffer::new();
    let mut total_train: u64 = 0;
    let mut total_val: u64 = 0;
    let total_failed: u64 = 0;

    while let Ok(batch) = rx.recv() {
        for pos in batch.positions {
            if pos.is_val {
                buf_val.add(pos.eval, pos.stm, pos.piece_count, pos.wdl, &pos.features);
                total_val += 1;
                if buf_val.len() as u64 >= flush_every {
                    val.flush(&buf_val)?;
                    buf_val.clear();
                }
            } else {
                buf_train.add(pos.eval, pos.stm, pos.piece_count, pos.wdl, &pos.features);
                total_train += 1;
                if buf_train.len() as u64 >= flush_every {
                    train.flush(&buf_train)?;
                    buf_train.clear();
                }
            }
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Final flush
    if buf_train.len() > 0 { train.flush(&buf_train)?; }
    if buf_val.len() > 0 { val.flush(&buf_val)?; }

    train_counter.store(total_train, Ordering::Relaxed);
    val_counter.store(total_val, Ordering::Relaxed);
    let _ = total_failed; // failed counter is updated by workers, not here

    // Write attributes (binpack_path becomes the source_binpack attr — input file)
    write_attributes(&h5, total_train, total_val, val_fraction, &binpack_path)?;
    h5.close()?;
    Ok(())
}

fn write_attributes(
    h5: &H5File,
    n_train: u64,
    n_val: u64,
    val_fraction: f64,
    binpack_path: &PathBuf,
) -> Result<()> {
    let attr_str = |name: &str, val: &str| -> Result<()> {
        let attr = h5
            .new_attr::<hdf5_metno::types::VarLenUnicode>()
            .shape(())
            .create(name)?;
        let s: hdf5_metno::types::VarLenUnicode = val.parse().unwrap();
        attr.write_scalar(&s)?;
        Ok(())
    };
    let attr_f32 = |name: &str, val: f32| -> Result<()> {
        let attr = h5.new_attr::<f32>().shape(()).create(name)?;
        attr.write_scalar(&val)?;
        Ok(())
    };
    let attr_u32 = |name: &str, val: u32| -> Result<()> {
        let attr = h5.new_attr::<u32>().shape(()).create(name)?;
        attr.write_scalar(&val)?;
        Ok(())
    };
    let attr_u64 = |name: &str, val: u64| -> Result<()> {
        let attr = h5.new_attr::<u64>().shape(()).create(name)?;
        attr.write_scalar(&val)?;
        Ok(())
    };
    let attr_bool = |name: &str, val: bool| -> Result<()> {
        let attr = h5.new_attr::<u8>().shape(()).create(name)?;
        let v: u8 = if val { 1 } else { 0 };
        attr.write_scalar(&v)?;
        Ok(())
    };

    attr_str("pipeline_version", PIPELINE_VERSION)?;
    attr_str("format_version", FORMAT_VERSION)?;
    attr_str("eval_perspective", EVAL_PERSPECTIVE)?;
    attr_f32("eval_scale", EVAL_SCALE)?;
    attr_f32("target_clip", TARGET_CLIP)?;
    attr_f32("mate_base", MATE_BASE)?;
    attr_f32("mate_decay", MATE_DECAY)?;
    attr_u32("min_depth", MIN_DEPTH)?;
    attr_f32("wdl_temperature", WDL_TEMPERATURE)?;
    attr_bool("mate_filter_relaxed", true)?;
    attr_u64("total", n_train + n_val)?;
    attr_u64("n_train", n_train)?;
    attr_u64("n_val", n_val)?;
    attr_f32("val_split", val_fraction as f32)?;
    attr_str(
        "source_binpack",
        binpack_path.file_name().unwrap().to_str().unwrap(),
    )?;
    Ok(())
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    let args = Args::parse();
    let num_workers = args.workers.unwrap_or_else(|| num_cpus::get().max(2).min(16));

    let binpack_size = std::fs::metadata(&args.binpack)
        .with_context(|| format!("binpack not found: {:?}", args.binpack))?
        .len();

    println!("=== binpack_to_h5 (Stockfish binpack -> HDF5 sparse_v1) ===");
    println!("Binpack:    {:?} ({:.2} GB)", args.binpack, binpack_size as f64 / 1e9);
    println!("Output:     {:?}", args.output);
    println!("Workers:    1 reader + {} encoders + 1 writer", num_workers);
    println!("Batch size: {} entries", BATCH_SIZE);
    println!("Channel cap: {} batches", args.channel_capacity);
    println!("Flush every: {} positions", args.flush_every);
    if let Some(max) = args.max_positions {
        println!("Max positions: {}", max);
    }
    println!();

    let target_total = args.max_positions.unwrap_or(u64::MAX);

    // Channels
    let (tx_read, rx_read) = bounded::<ReadBatch>(args.channel_capacity);
    let (tx_encoded, rx_encoded) = bounded::<EncodedBatch>(args.channel_capacity);

    let counter = Arc::new(AtomicU64::new(0));
    let failed_counter = Arc::new(AtomicU64::new(0));
    let filtered_counter = Arc::new(AtomicU64::new(0));
    let train_counter = Arc::new(AtomicU64::new(0));
    let val_counter = Arc::new(AtomicU64::new(0));

    let t_start = Instant::now();

    // === Spawn writer thread ===
    let writer_output = args.output.clone();
    let writer_binpack = args.binpack.clone();
    let writer_counter = counter.clone();
    let writer_train = train_counter.clone();
    let writer_val = val_counter.clone();
    let writer_flush_every = args.flush_every;
    let writer_val_fraction = args.val_fraction;
    let writer_handle = thread::spawn(move || -> Result<()> {
        writer_thread(
            writer_output,
            writer_binpack,
            rx_encoded,
            writer_flush_every,
            writer_counter,
            writer_train,
            writer_val,
            writer_val_fraction,
        )
    });

    // === Spawn N encoder workers ===
    let mut encoder_handles = Vec::new();
    for _ in 0..num_workers {
        let rx = rx_read.clone();
        let tx = tx_encoded.clone();
        let failed = failed_counter.clone();
        let filtered = filtered_counter.clone();
        let val_frac = args.val_fraction;
        let h = thread::spawn(move || -> Result<()> {
            encoder_worker(rx, tx, val_frac, failed, filtered)
        });
        encoder_handles.push(h);
    }
    drop(rx_read);
    drop(tx_encoded); // only worker clones remain

    // === Spawn progress thread ===
    let counter_pb = counter.clone();
    let pb_done = Arc::new(AtomicU64::new(0));
    let pb_done_clone = pb_done.clone();
    let pb_handle = thread::spawn(move || {
        let pb = ProgressBar::new(if target_total == u64::MAX { 0 } else { target_total });
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>11} ({per_sec})",
            )
            .unwrap(),
        );
        loop {
            let n = counter_pb.load(Ordering::Relaxed);
            pb.set_position(n);
            if pb_done_clone.load(Ordering::Relaxed) == 1 {
                pb.finish();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    // === Reader (main thread) ===
    let file = File::open(&args.binpack).context("failed to open binpack")?;
    let mut reader = CompressedTrainingDataEntryReader::new(file)
        .map_err(|e| anyhow::anyhow!("CompressedTrainingDataEntryReader failed: {:?}", e))?;

    let mut batch: Vec<sfbinpack::TrainingDataEntry> = Vec::with_capacity(BATCH_SIZE);
    let mut total_read: u64 = 0;
    let mut next_base_idx: u64 = 0;

    while reader.has_next() {
        if total_read >= target_total {
            break;
        }
        let entry = reader.next();
        batch.push(entry);
        total_read += 1;
        if batch.len() >= BATCH_SIZE {
            let item = ReadBatch {
                entries: std::mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE)),
                base_idx: next_base_idx,
            };
            next_base_idx += BATCH_SIZE as u64;
            tx_read.send(item).ok();
        }
    }
    if !batch.is_empty() {
        let item = ReadBatch {
            entries: batch,
            base_idx: next_base_idx,
        };
        tx_read.send(item).ok();
    }
    drop(tx_read); // signals EOF to encoder workers

    // === Wait for encoders ===
    for h in encoder_handles {
        match h.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("encoder worker ERROR: {:?}", e),
            Err(e) => eprintln!("encoder worker PANIC: {:?}", e),
        }
    }

    // === Wait for writer (finishes once tx_encoded clones drop) ===
    match writer_handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(e) => return Err(anyhow::anyhow!("writer panic: {:?}", e)),
    }

    pb_done.store(1, Ordering::Relaxed);
    let _ = pb_handle.join();

    let elapsed = t_start.elapsed();
    let total_train = train_counter.load(Ordering::Relaxed);
    let total_val = val_counter.load(Ordering::Relaxed);
    let total_failed = failed_counter.load(Ordering::Relaxed);
    let total_filtered = filtered_counter.load(Ordering::Relaxed);
    let total_proc = total_train + total_val;
    let total_seen = total_proc + total_failed + total_filtered;
    let filter_pct = if total_seen > 0 {
        100.0 * total_filtered as f64 / total_seen as f64
    } else { 0.0 };
    let rate = total_proc as f64 / elapsed.as_secs_f64();
    let out_size = std::fs::metadata(&args.output).map(|m| m.len()).unwrap_or(0);

    println!();
    println!("=== CONVERSION COMPLETE ===");
    println!("Time:      {:.1} min ({:.1} sec)", elapsed.as_secs_f64() / 60.0, elapsed.as_secs_f64());
    println!("Train:     {} positions", total_train);
    println!("Val:       {} positions", total_val);
    println!("Filtered:  {} positions ({:.1}% - captures+in-check)", total_filtered, filter_pct);
    println!("Failed:    {} positions", total_failed);
    println!("Rate:      {:.0} positions/sec (kept) / {:.0} (raw read)",
             rate, total_seen as f64 / elapsed.as_secs_f64());
    println!("Output:    {:?} ({:.2} GB)", args.output, out_size as f64 / 1e9);

    Ok(())
}
