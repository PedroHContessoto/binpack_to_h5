//! Pipeline-wide constants written as HDF5 attributes so consumers can
//! validate the data they're loading.

/// Identifies the pipeline that produced the file. Bump on schema changes.
pub const PIPELINE_VERSION: &str = "sf_binpack_halfp_v1";

/// Schema version of the per-group dataset layout (sparse_v1 = current).
pub const FORMAT_VERSION: &str = "sparse_v1";

/// Indicates `evals` and `wdl` are stored from white's perspective, regardless
/// of `stm`. Consumers can flip when computing STM-relative loss.
pub const EVAL_PERSPECTIVE: &str = "white_pov_assumed";

/// Divisor applied to centipawn scores before storage:
///   stored_eval = clip(cp / EVAL_SCALE, ±TARGET_CLIP)  for normal evals
///   stored_eval = sign * (MATE_BASE - ply_decay)        for mate scores
///
/// 400.0 is the Stockfish-baseline value. Lower (e.g., 300) gives finer
/// resolution around equal positions but more saturation on decisive ones.
pub const EVAL_SCALE: f32 = 400.0;

/// Hard clip applied to normal (non-mate) eval magnitudes after scaling.
pub const TARGET_CLIP: f32 = 6.5;

/// Base value used to encode mate scores (anchored above TARGET_CLIP).
pub const MATE_BASE: f32 = 10.0;

/// Per-ply decay applied to mate encodings — closer mates get higher values.
pub const MATE_DECAY: f32 = 0.1;

/// Centipawn threshold above which scores are treated as mate encodings.
pub const MATE_THRESHOLD_CP: i32 = 25_000;

/// Documentation-only attribute: minimum search depth the source data was
/// supposed to have been generated at. Not enforced.
pub const MIN_DEPTH: u32 = 9;

/// Documentation-only attribute: temperature used by downstream WDL targets.
/// Not applied to the stored values.
pub const WDL_TEMPERATURE: f32 = 200.0;

/// Number of binpack entries the reader bundles per channel message.
pub const BATCH_SIZE: usize = 8192;
