//! Encoder capability types. Detection is parsed from FFmpeg text, never guessed.

mod capabilities;

mod select;

pub use capabilities::{
    capabilities_from_encoder_list, parse_ffmpeg_encoders, Capability, EncoderCapabilities,
    FfmpegEncoderLine, VideoEncoder,
};
pub use select::{listed_hardware, resolve_encoder, tune_for, EncodeTune};
