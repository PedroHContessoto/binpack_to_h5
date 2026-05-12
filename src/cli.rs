//! Command-line argument parsing.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Convert Stockfish binpack to HDF5 sparse_v1 (HalfP white-POV)")]
pub struct Args {
    /// Input .binpack path
    #[arg(long)]
    pub binpack: PathBuf,

    /// Output .h5 path (single file, will be overwritten)
    #[arg(long)]
    pub output: PathBuf,

    /// Encoder worker count (default = num_cpus, 16 recommended max)
    #[arg(long)]
    pub workers: Option<usize>,

    /// Cap on raw positions read from binpack (default: read all)
    #[arg(long)]
    pub max_positions: Option<u64>,

    /// Fraction held out for the val split (default 0.005 = 0.5%)
    #[arg(long, default_value_t = 0.005)]
    pub val_fraction: f64,

    /// Flush HDF5 every N accumulated positions (default 500k)
    #[arg(long, default_value_t = 500_000)]
    pub flush_every: u64,

    /// Channel buffer size, in batches of BATCH_SIZE entries each
    #[arg(long, default_value_t = 16)]
    pub channel_capacity: usize,
}
