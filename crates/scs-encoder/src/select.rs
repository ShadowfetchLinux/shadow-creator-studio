use scs_core::quality::QualityPreset;
use scs_core::settings::EncoderPreference;

use crate::capabilities::{EncoderCapabilities, VideoEncoder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeTune {
    pub bitrate: u32,
    pub maxrate: u32,
    pub cq: u32,
    pub preset: &'static str,
    pub software_preset: &'static str,
    pub crf: u32,
}

pub fn resolve_encoder(
    pref: EncoderPreference,
    caps: &EncoderCapabilities,
) -> Result<VideoEncoder, String> {
    let pick = |wanted: VideoEncoder, available: bool, name: &str| {
        if available {
            Ok(wanted)
        } else {
            Err(format!(
                "{name} is not listed by FFmpeg, so it cannot be used."
            ))
        }
    };
    match pref {
        EncoderPreference::NvencH264 => pick(
            VideoEncoder::H264Nvenc,
            caps.h264_nvenc.is_available(),
            "h264_nvenc",
        ),
        EncoderPreference::NvencHevc => pick(
            VideoEncoder::HevcNvenc,
            caps.hevc_nvenc.is_available(),
            "hevc_nvenc",
        ),
        EncoderPreference::NvencAv1 => pick(
            VideoEncoder::Av1Nvenc,
            caps.av1_nvenc.is_available(),
            "av1_nvenc",
        ),
        EncoderPreference::SoftwareX264 => pick(
            VideoEncoder::Libx264,
            caps.libx264.is_available(),
            "libx264",
        ),
        EncoderPreference::AutoNvenc => {
            if let Some(enc) = caps.preferred_nvenc() {
                Ok(enc)
            } else if caps.libx264.is_available() {
                Ok(VideoEncoder::Libx264)
            } else {
                Err("No usable encoder is listed (need h264_nvenc or libx264).".into())
            }
        }
    }
}

pub fn tune_for(quality: QualityPreset, encoder: VideoEncoder) -> EncodeTune {
    let (bitrate, cq, crf) = match quality {
        QualityPreset::Youtube1080p30 => (12_000_000, 23, 20),
        QualityPreset::Youtube1080p60 => (16_000_000, 21, 18),
        QualityPreset::Youtube1440p60 => (24_000_000, 21, 18),
        QualityPreset::Youtube4k30 => (35_000_000, 20, 18),
        QualityPreset::High => (40_000_000, 18, 16),
        QualityPreset::Balanced => (6_000_000, 28, 26),
        QualityPreset::Custom => (16_000_000, 23, 20),
    };
    let hardware = !matches!(encoder, VideoEncoder::Libx264);
    EncodeTune {
        bitrate,
        maxrate: bitrate.saturating_add(bitrate / 4),
        cq,
        preset: if hardware { "p4" } else { "medium" },
        software_preset: if matches!(quality, QualityPreset::Balanced) {
            "veryfast"
        } else {
            "fast"
        },
        crf,
    }
}

pub fn listed_hardware(caps: &EncoderCapabilities) -> Vec<&'static str> {
    let mut out = Vec::new();
    if caps.h264_nvenc.is_available() {
        out.push("h264_nvenc");
    }
    if caps.hevc_nvenc.is_available() {
        out.push("hevc_nvenc");
    }
    if caps.av1_nvenc.is_available() {
        out.push("av1_nvenc");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::capabilities_from_encoder_list;

    const BOTH: &str = "\
Encoders:
 V..... = Video
 ------
 V....D h264_nvenc           NVIDIA NVENC H.264 encoder (codec h264)
 V....D libx264              libx264 H.264 (codec h264)
";

    #[test]
    fn auto_prefers_nvenc_then_software() {
        let caps = capabilities_from_encoder_list(BOTH);
        assert_eq!(
            resolve_encoder(EncoderPreference::AutoNvenc, &caps).unwrap(),
            VideoEncoder::H264Nvenc
        );
        let soft = capabilities_from_encoder_list(" V....D libx264              x264 (codec h264)\n");
        assert_eq!(
            resolve_encoder(EncoderPreference::AutoNvenc, &soft).unwrap(),
            VideoEncoder::Libx264
        );
    }

    #[test]
    fn explicit_missing_encoder_is_an_error() {
        let caps = capabilities_from_encoder_list(" V....D libx264              x264 (codec h264)\n");
        let err = resolve_encoder(EncoderPreference::NvencAv1, &caps).unwrap_err();
        assert!(err.contains("av1_nvenc"));
    }

    #[test]
    fn quality_maps_bitrate_and_presets() {
        let high = tune_for(QualityPreset::Youtube1080p60, VideoEncoder::H264Nvenc);
        assert_eq!(high.bitrate, 16_000_000);
        assert_eq!(high.preset, "p4");
        let small = tune_for(QualityPreset::Balanced, VideoEncoder::Libx264);
        assert!(small.bitrate < high.bitrate);
        assert_eq!(small.software_preset, "veryfast");
        let archival = tune_for(QualityPreset::High, VideoEncoder::HevcNvenc);
        assert!(archival.bitrate > high.bitrate);
    }
}
