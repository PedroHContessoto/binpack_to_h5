//! `binpack_to_h5` — convert a Stockfish binpack into HDF5 sparse_v1.
//!
//! Multi-threaded pipeline:
//! ```text
//!   1 reader (main thread) ─▶ N encoders ─▶ 1 writer thread
//! ```
//! - Reader: decompresses binpack via `sfbinpack` and batches entries
//! - Encoders: Stockfish-style filter + HalfP encoding (parallel)
//! - Writer: appends to extensible HDF5 datasets (sequential, HDF5 not
//!   thread-safe)
//!
//! See `cargo run --release -- --help` and the README for usage.

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::bounded;
use indicatif::{ProgressBar, ProgressStyle};
use sfbinpack::CompressedTrainingDataEntryReader;
use std::fs::File;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use binpack_to_h5::cli::Args;
use binpack_to_h5::config::BATCH_SIZE;
use binpack_to_h5::h5_writer::writer_thread;
use binpack_to_h5::pipeline::{encoder_worker, EncodedBatch, ReadBatch};

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

    // === Channels ===
    let (tx_read, rx_read) = bounded::<ReadBatch>(args.channel_capacity);
    let (tx_encoded, rx_encoded) = bounded::<EncodedBatch>(args.channel_capacity);

    // === Counters ===
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
    let mut read_panics: u64 = 0;

    while reader.has_next() {
        if total_read >= target_total {
            break;
        }
        // sfbinpack 0.6.x can panic during entry deserialization on certain
        // binpacks (corrupted compressed positions / move squares). Catch the
        // panic, count it, and try to keep reading the rest of the file.
        let entry = match std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| reader.next()),
        ) {
            Ok(e) => e,
            Err(_) => {
                read_panics += 1;
                if read_panics > 10_000 {
                    eprintln!("ERROR: too many read panics (>10000), aborting");
                    break;
                }
                continue;
            }
        };
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

    // === Final report ===
    let elapsed = t_start.elapsed();
    let total_train = train_counter.load(Ordering::Relaxed);
    let total_val = val_counter.load(Ordering::Relaxed);
    let total_failed = failed_counter.load(Ordering::Relaxed);
    let total_filtered = filtered_counter.load(Ordering::Relaxed);
    let total_proc = total_train + total_val;
    let total_seen = total_proc + total_failed + total_filtered;
    let filter_pct = if total_seen > 0 {
        100.0 * total_filtered as f64 / total_seen as f64
    } else {
        0.0
    };
    let rate = total_proc as f64 / elapsed.as_secs_f64();
    let out_size = std::fs::metadata(&args.output).map(|m| m.len()).unwrap_or(0);

    println!();
    println!("=== CONVERSION COMPLETE ===");
    println!("Time:      {:.1} min ({:.1} sec)", elapsed.as_secs_f64() / 60.0, elapsed.as_secs_f64());
    println!("Train:     {} positions", total_train);
    println!("Val:       {} positions", total_val);
    println!("Filtered:  {} positions ({:.1}% - captures+in-check)", total_filtered, filter_pct);
    println!("Failed:    {} positions", total_failed);
    if read_panics > 0 {
        println!("Skipped:   {} entries (sfbinpack deserialization panic)", read_panics);
    }
    println!(
        "Rate:      {:.0} positions/sec (kept) / {:.0} (raw read)",
        rate,
        total_seen as f64 / elapsed.as_secs_f64()
    );
    println!("Output:    {:?} ({:.2} GB)", args.output, out_size as f64 / 1e9);

    Ok(())
}
