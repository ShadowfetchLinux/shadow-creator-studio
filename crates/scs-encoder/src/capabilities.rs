use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Capability {
    Available { notes: String },
    Unavailable { reason: String },
    Unknown,
}

impl Capability {
    pub fn from_present(present: bool, available_notes: &str, missing: &str) -> Self {
        if present {
            Self::Available {
                notes: available_notes.to_string(),
            }
        } else {
            Self::Unavailable {
                reason: missing.to_string(),
            }
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderCapabilities {
    pub h264_nvenc: Capability,
    pub hevc_nvenc: Capability,
    pub av1_nvenc: Capability,
    pub libx264: Capability,
    pub libx265: Capability,
}

impl EncoderCapabilities {
    pub fn unknown() -> Self {
        Self {
            h264_nvenc: Capability::Unknown,
            hevc_nvenc: Capability::Unknown,
            av1_nvenc: Capability::Unknown,
            libx264: Capability::Unknown,
            libx265: Capability::Unknown,
        }
    }

    pub fn preferred_nvenc(&self) -> Option<VideoEncoder> {
        if self.h264_nvenc.is_available() {
            Some(VideoEncoder::H264Nvenc)
        } else if self.hevc_nvenc.is_available() {
            Some(VideoEncoder::HevcNvenc)
        } else if self.av1_nvenc.is_available() {
            Some(VideoEncoder::Av1Nvenc)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoEncoder {
    H264Nvenc,
    HevcNvenc,
    Av1Nvenc,
    Libx264,
}

impl VideoEncoder {
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::H264Nvenc => "h264_nvenc",
            Self::HevcNvenc => "hevc_nvenc",
            Self::Av1Nvenc => "av1_nvenc",
            Self::Libx264 => "libx264",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegEncoderLine {
    pub name: String,
    pub codec: Option<String>,
    pub description: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_hardware: bool,
}

/// Parse `ffmpeg -encoders` human output into typed rows.
pub fn parse_ffmpeg_encoders(text: &str) -> Vec<FfmpegEncoderLine> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.len() < 10 {
            continue;
        }
        let flags = &line[..8.min(line.len())];
        let is_video = flags.contains('V');
        let is_audio = flags.contains('A');
        if !is_video && !is_audio {
            continue;
        }
        let rest = line[8.min(line.len())..].trim();
        let mut bits = rest.splitn(2, |c: char| c.is_whitespace());
        let Some(name) = bits.next().filter(|n| !n.is_empty()) else {
            continue;
        };
        if name == "=" || name.starts_with('-') || name == "------" {
            continue;
        }
        let description = bits.next().unwrap_or("").trim().to_string();
        let codec = description
            .rsplit_once("(codec ")
            .and_then(|(_, rest)| rest.strip_suffix(')'))
            .map(ToOwned::to_owned);
        out.push(FfmpegEncoderLine {
            is_hardware: name.contains("nvenc")
                || name.contains("vaapi")
                || name.contains("qsv")
                || name.contains("v4l2"),
            name: name.to_string(),
            codec,
            description,
            is_video,
            is_audio,
        });
    }
    out
}

pub fn capabilities_from_encoder_list(text: &str) -> EncoderCapabilities {
    let lines = parse_ffmpeg_encoders(text);
    let has = |name: &str| lines.iter().any(|line| line.name == name);
    EncoderCapabilities {
        h264_nvenc: Capability::from_present(
            has("h264_nvenc"),
            "FFmpeg listed h264_nvenc",
            "h264_nvenc not listed by FFmpeg",
        ),
        hevc_nvenc: Capability::from_present(
            has("hevc_nvenc"),
            "FFmpeg listed hevc_nvenc",
            "hevc_nvenc not listed by FFmpeg",
        ),
        av1_nvenc: Capability::from_present(
            has("av1_nvenc"),
            "FFmpeg listed av1_nvenc",
            "av1_nvenc not listed by FFmpeg",
        ),
        libx264: Capability::from_present(
            has("libx264"),
            "FFmpeg listed libx264",
            "libx264 not listed by FFmpeg",
        ),
        libx265: Capability::from_present(
            has("libx265"),
            "FFmpeg listed libx265",
            "libx265 not listed by FFmpeg",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
Encoders:
 V..... = Video
 A..... = Audio
 ------
 V....D libaom-av1           libaom AV1 (codec av1)
 V....D av1_nvenc            NVIDIA NVENC av1 encoder (codec av1)
 V....D libx264              libx264 H.264 (codec h264)
 V....D h264_nvenc           NVIDIA NVENC H.264 encoder (codec h264)
 V....D libx265              libx265 H.265 / HEVC (codec hevc)
 V....D hevc_nvenc           NVIDIA NVENC hevc encoder (codec hevc)
 A....D aac                  AAC (Advanced Audio Coding)
";

    #[test]
    fn parses_nvenc_and_software() {
        let lines = parse_ffmpeg_encoders(FIXTURE);
        assert!(lines.iter().any(|l| l.name == "h264_nvenc" && l.is_hardware));
        assert!(lines.iter().any(|l| l.name == "aac" && l.is_audio));
        let caps = capabilities_from_encoder_list(FIXTURE);
        assert!(caps.h264_nvenc.is_available());
        assert!(caps.av1_nvenc.is_available());
        assert_eq!(caps.preferred_nvenc(), Some(VideoEncoder::H264Nvenc));
        assert_eq!(VideoEncoder::H264Nvenc.ffmpeg_name(), "h264_nvenc");
    }

    #[test]
    fn missing_encoders_are_unavailable() {
        let caps = capabilities_from_encoder_list(" V....D libx264              x264 (codec h264)\n");
        assert!(!caps.h264_nvenc.is_available());
        assert!(caps.libx264.is_available());
        match caps.h264_nvenc {
            Capability::Unavailable { .. } => {}
            other => panic!("{other:?}"),
        }
    }
}
