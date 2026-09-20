//! Typed FFmpeg argv construction. Callers must never wrap this in a shell.

mod builder;

pub use builder::{FfmpegCommandBuilder, PlannedCommand, RemuxPlan};
