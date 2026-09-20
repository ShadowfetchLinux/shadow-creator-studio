//! Typed FFmpeg argv construction. Callers must never wrap this in a shell.

mod builder;
mod preview;

pub use builder::{FfmpegCommandBuilder, PlannedCommand, RemuxPlan};
pub use preview::camera_preview_rgb;
