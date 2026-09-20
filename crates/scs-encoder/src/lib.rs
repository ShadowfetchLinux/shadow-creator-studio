//! Encoder capability types. Detection is parsed from FFmpeg text, never guessed.

mod capabilities;

pub use capabilities::{
    capabilities_from_encoder_list, parse_ffmpeg_encoders, Capability, EncoderCapabilities,
    FfmpegEncoderLine, VideoEncoder,
};
