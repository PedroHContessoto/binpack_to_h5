//! `binpack_to_h5` — convert Stockfish binpack files into HDF5 sparse_v1.
//!
//! Library API. The `binpack_to_h5` binary in `src/main.rs` is a thin
//! orchestration layer over these modules.
//!
//! Module map:
//! - [`config`]: pipeline-wide constants written into HDF5 attributes
//! - [`cli`]: command-line argument struct (Clap derive)
//! - [`filter`]: Stockfish-style entry filtering (captures + in-check)
//! - [`encoding`]: HalfP feature encoding + centipawn normalization
//! - [`pipeline`]: channel message types and the encoder worker thread
//! - [`h5_writer`]: HDF5 dataset wrapper and writer thread

pub mod cli;
pub mod config;
pub mod encoding;
pub mod filter;
pub mod h5_writer;
pub mod pipeline;
