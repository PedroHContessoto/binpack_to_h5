//! HDF5 writer thread + on-disk dataset wrapper + attribute writer.
//!
//! All HDF5 calls happen on this single thread (the bundled HDF5 library is
//! not thread-safe). The encoder workers send fully-encoded batches; this
//! module accumulates them in a `PositionBuffer` and flushes to extensible
//! HDF5 datasets in chunks.

use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use hdf5_metno::{Dataset, File as H5File, Group};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::config::{
    EVAL_PERSPECTIVE, EVAL_SCALE, FORMAT_VERSION, MATE_BASE, MATE_DECAY, MIN_DEPTH,
    PIPELINE_VERSION, TARGET_CLIP, WDL_TEMPERATURE,
};
use crate::pipeline::EncodedBatch;

/// In-memory accumulator for one split (train or val) before flushing to HDF5.
pub struct PositionBuffer {
    pub evals: Vec<f32>,
    pub has_eval: Vec<u8>,
    pub has_wdl: Vec<u8>,
    pub stm: Vec<u8>,
    pub piece_count: Vec<u8>,
    pub wdl: Vec<f32>,
    pub w_flat: Vec<i16>,
    /// Offsets relative to this buffer; turned into absolute offsets on flush.
    pub offsets_relative: Vec<i64>,
}

impl PositionBuffer {
    pub fn new() -> Self {
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

    pub fn len(&self) -> usize {
        self.evals.len()
    }

    pub fn add(&mut self, ev: f32, st: u8, pc: u8, wdl_val: f32, features: &[i16]) {
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

    pub fn clear(&mut self) {
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

impl Default for PositionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Wraps the extensible HDF5 datasets for one split (train or val) and
/// tracks how many positions / features have been written so far.
pub struct H5Group {
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
    pub fn create(parent: &Group) -> Result<Self> {
        let chunk_pos = 65_536_usize;
        let chunk_w = 1_usize << 20;

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

    pub fn flush(&mut self, buf: &PositionBuffer) -> Result<()> {
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
        let offset_start = if self.n_written == 0 {
            0
        } else {
            self.n_written + 1
        };
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

/// Writer thread body. Owns the HDF5 file; receives `EncodedBatch`es and
/// flushes to disk in chunks of `flush_every` positions per split.
#[allow(clippy::too_many_arguments)]
pub fn writer_thread(
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

    if buf_train.len() > 0 {
        train.flush(&buf_train)?;
    }
    if buf_val.len() > 0 {
        val.flush(&buf_val)?;
    }

    train_counter.store(total_train, Ordering::Relaxed);
    val_counter.store(total_val, Ordering::Relaxed);

    write_attributes(&h5, total_train, total_val, val_fraction, &binpack_path)?;
    h5.close()?;
    Ok(())
}

/// Writes pipeline-identifying attributes at the file root. Consumers should
/// validate `pipeline_version`/`format_version` before loading the data.
pub fn write_attributes(
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
