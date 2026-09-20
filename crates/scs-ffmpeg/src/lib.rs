//! Typed FFmpeg argv construction. Callers must never wrap this in a shell.

mod builder;
mod preview;
mod progress;
mod record;
mod remux;

pub use builder::{FfmpegCommandBuilder, PlannedCommand, RemuxPlan};
pub use preview::camera_preview_rgb;
pub use progress::{human_ffmpeg_error, parse_drop_from_stats, parse_progress_block, FfmpegProgress};
pub use record::{plan_record, sidecar_path, CameraInput, RecordPlanRequest};
pub use remux::{may_delete_mkv, parse_probe_default, remux_command, verify_media, RemuxVerify};
