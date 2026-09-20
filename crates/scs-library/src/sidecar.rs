use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sidecar {
    pub title: Option<String>,
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub size_bytes: Option<u64>,
    pub thumbnail_path: Option<PathBuf>,
}

pub fn sidecar_path(media: impl AsRef<Path>) -> PathBuf {
    media.as_ref().with_extension("json")
}

pub fn load_sidecar(path: impl AsRef<Path>) -> Option<Sidecar> {
    let raw = std::fs::read_to_string(path.as_ref()).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn write_sidecar(media: impl AsRef<Path>, sidecar: &Sidecar) -> Result<PathBuf, String> {
    let path = sidecar_path(media);
    let json = serde_json::to_string_pretty(sidecar).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn parse_ffprobe_json(text: &str) -> Result<Sidecar, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "ffprobe JSON was not valid.".to_string())?;
    let format = value.get("format").cloned().unwrap_or(serde_json::Value::Null);
    let duration_secs = format
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| format.get("duration").and_then(|v| v.as_f64()));
    let size_bytes = format
        .get("size")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| format.get("size").and_then(|v| v.as_u64()));
    let mut sidecar = Sidecar {
        duration_secs,
        size_bytes,
        ..Sidecar::default()
    };
    if let Some(streams) = value.get("streams").and_then(|v| v.as_array()) {
        for stream in streams {
            let codec = stream
                .get("codec_name")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            match stream.get("codec_type").and_then(|v| v.as_str()) {
                Some("video") => {
                    sidecar.video_codec = codec;
                    sidecar.width = stream.get("width").and_then(|v| v.as_u64()).map(|n| n as u32);
                    sidecar.height = stream.get("height").and_then(|v| v.as_u64()).map(|n| n as u32);
                    sidecar.fps = stream
                        .get("avg_frame_rate")
                        .and_then(|v| v.as_str())
                        .and_then(parse_rate);
                }
                Some("audio") if sidecar.audio_codec.is_none() => sidecar.audio_codec = codec,
                _ => {}
            }
        }
    }
    Ok(sidecar)
}

fn parse_rate(rate: &str) -> Option<f32> {
    if let Some((n, d)) = rate.split_once('/') {
        let n: f32 = n.parse().ok()?;
        let d: f32 = d.parse().ok()?;
        if d == 0.0 {
            return None;
        }
        Some(n / d)
    } else {
        rate.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_path_swaps_extension() {
        assert_eq!(
            sidecar_path("/rec/a.mkv"),
            PathBuf::from("/rec/a.json")
        );
    }

    #[test]
    fn parse_probe_json() {
        let json = r#"{
            "format": {"duration": "12.5", "size": "2048"},
            "streams": [
                {"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"avg_frame_rate":"60/1"},
                {"codec_type":"audio","codec_name":"aac"}
            ]
        }"#;
        let side = parse_ffprobe_json(json).unwrap();
        assert!((side.duration_secs.unwrap() - 12.5).abs() < 0.01);
        assert_eq!(side.width, Some(1920));
        assert_eq!(side.fps, Some(60.0));
        assert_eq!(side.video_codec.as_deref(), Some("h264"));
        assert_eq!(side.audio_codec.as_deref(), Some("aac"));
    }
}
