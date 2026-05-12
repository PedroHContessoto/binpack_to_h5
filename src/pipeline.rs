//! Inter-thread message types and the encoder worker.
//!
//! Pipeline:
//! ```text
//!  reader (main) ──ReadBatch──▶ N encoders ──EncodedBatch──▶ writer
//! ```

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::encoding::{encode_halfp, normalize_eval};
use crate::filter::{is_malformed_entry, should_skip_entry};

/// Reader -> encoder workers.
pub struct ReadBatch {
    pub entries: Vec<sfbinpack::TrainingDataEntry>,
    pub base_idx: u64,
}

/// One fully encoded position (worker -> writer).
pub struct EncodedPos {
    pub eval: f32,
    pub stm: u8,
    pub piece_count: u8,
    pub wdl: f32,
    pub features: Vec<i16>,
    pub is_val: bool,
}

/// Batch of encoded positions sent to the writer.
pub struct EncodedBatch {
    pub positions: Vec<EncodedPos>,
}

enum EntryOutcome {
    Encoded(EncodedPos),
    Filtered,
    Failed,
}

/// Encoder worker thread body. Pulls `ReadBatch`es, applies the Stockfish-style
/// filter, encodes HalfP features, and forwards encoded batches downstream.
///
/// Per-entry processing is wrapped in `catch_unwind` because some binpacks
/// trigger sfbinpack 0.6.x panics deep inside `Position::piece_at` /
/// `is_checked`. A bad entry is counted as failed; the loop continues.
pub fn encoder_worker(
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
            let global_idx = item.base_idx + i as u64;

            // Move the reusable feature buffer into the closure so
            // catch_unwind sees no shared mutable state.
            let mut local_features: Vec<i16> = Vec::new();
            std::mem::swap(&mut features, &mut local_features);

            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if is_malformed_entry(entry) {
                    return EntryOutcome::Failed;
                }
                if should_skip_entry(entry) {
                    return EntryOutcome::Filtered;
                }
                encode_halfp(&entry.pos, &mut local_features);
                if local_features.is_empty() {
                    return EntryOutcome::Failed;
                }
                let pc = local_features.len() as u8;
                let stm_val = entry.pos.side_to_move().ordinal();
                let stm_is_white = stm_val == 0;
                let raw_score = entry.score as i32;
                let raw_result = entry.result as f32;
                let score_white_pov = if stm_is_white { raw_score } else { -raw_score };
                let result_white_pov = if stm_is_white { raw_result } else { -raw_result };
                let ev = normalize_eval(score_white_pov);
                let is_val = (global_idx % 1000) < val_threshold;
                EntryOutcome::Encoded(EncodedPos {
                    eval: ev,
                    stm: stm_val,
                    piece_count: pc,
                    wdl: result_white_pov,
                    features: local_features.clone(),
                    is_val,
                })
            }));

            // Restore feature buffer ownership for the next iteration.
            std::mem::swap(&mut features, &mut local_features);
            features.clear();

            match outcome {
                Ok(EntryOutcome::Encoded(pos)) => encoded_positions.push(pos),
                Ok(EntryOutcome::Filtered) => local_filtered += 1,
                Ok(EntryOutcome::Failed) | Err(_) => local_failed += 1,
            }
        }

        if !encoded_positions.is_empty() {
            if tx_encoded
                .send(EncodedBatch {
                    positions: encoded_positions,
                })
                .is_err()
            {
                break; // writer hung up
            }
        }
    }

    failed_counter.fetch_add(local_failed, Ordering::Relaxed);
    filtered_counter.fetch_add(local_filtered, Ordering::Relaxed);
    Ok(())
}
