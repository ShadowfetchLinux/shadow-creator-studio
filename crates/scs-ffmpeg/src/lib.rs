//! Typed FFmpeg argv construction. Callers must never wrap this in a shell.

mod builder;
mod preview;
mod progress;
mod record;
mod remux;
mod tools;

pub use builder::{FfmpegCommandBuilder, PlannedCommand, RemuxPlan};
pub use preview::camera_preview_rgb;
pub use progress::{human_ffmpeg_error, parse_drop_from_stats, parse_progress_block, FfmpegProgress};
pub use record::{plan_record, sidecar_path, CameraInput, RecordPlanRequest};
pub use remux::{may_delete_mkv, parse_probe_default, remux_command, verify_media, RemuxVerify};
pub use tools::{
    change_resolution, compress, extract_audio, normalize_audio, probe_command, remove_section,
    remux_mp4, scale_9x16, silence_detect_args, thumbnail, to_gif, to_mp3, to_wav, trim,
    youtube_ready_mp4, ToolJob,
};
