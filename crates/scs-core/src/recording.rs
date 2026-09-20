use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modes::RecordingMode;
use crate::quality::QualityPreset;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerFormat {
    #[default]
    Mkv,
    Mp4,
}

impl ContainerFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mkv => "mkv",
            Self::Mp4 => "mp4",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioTrackMetadata {
    pub index: u32,
    pub role: AudioTrackRole,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioTrackRole {
    Microphone,
    Desktop,
    Mixed,
    Music,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoStreamMetadata {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub codec: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderMetadata {
    pub name: String,
    pub engine: String,
    pub hardware: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub id: String,
    pub project: String,
    pub title: String,
    pub mode: RecordingMode,
    pub quality: QualityPreset,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub container: ContainerFormat,
    pub video_path: PathBuf,
    pub remux_path: Option<PathBuf>,
    pub remux_verified: bool,
    pub audio_tracks: Vec<AudioTrackMetadata>,
    pub video: VideoStreamMetadata,
    pub encoder: EncoderMetadata,
    pub markers_path: Option<PathBuf>,
    pub notes: String,
}

/// Accidental stop is too easy on a live take. Default is confirm; Settings can opt out.
pub fn stop_requires_confirmation() -> bool {
    true
}

pub fn stop_requires_confirmation_pref(confirm: bool) -> bool {
    confirm
}

impl RecordingMetadata {
    pub fn new(
        project: impl Into<String>,
        title: impl Into<String>,
        mode: RecordingMode,
        quality: QualityPreset,
        video_path: PathBuf,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project: project.into(),
            title: title.into(),
            mode,
            quality,
            started_at: Utc::now(),
            ended_at: None,
            container: ContainerFormat::Mkv,
            video_path,
            remux_path: None,
            remux_verified: false,
            audio_tracks: Vec::new(),
            video: VideoStreamMetadata {
                width: quality.width(),
                height: quality.height(),
                fps: quality.fps(),
                codec: "h264_nvenc".into(),
            },
            encoder: EncoderMetadata {
                name: "h264_nvenc".into(),
                engine: "unassigned".into(),
                hardware: true,
            },
            markers_path: None,
            notes: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn metadata_round_trip() {
        let mut meta = RecordingMetadata::new(
            "YouTube",
            "Record",
            RecordingMode::Creator,
            QualityPreset::Youtube1080p60,
            Path::new("/tmp/2026-09-20_YouTube_Record_001.mkv").to_path_buf(),
        );
        meta.audio_tracks.push(AudioTrackMetadata {
            index: 0,
            role: AudioTrackRole::Microphone,
            codec: "aac".into(),
            sample_rate: 48_000,
            channels: 1,
        });
        meta.audio_tracks.push(AudioTrackMetadata {
            index: 1,
            role: AudioTrackRole::Desktop,
            codec: "aac".into(),
            sample_rate: 48_000,
            channels: 2,
        });
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let back: RecordingMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project, "YouTube");
        assert_eq!(back.mode, RecordingMode::Creator);
        assert_eq!(back.audio_tracks.len(), 2);
        assert!(!back.remux_verified);
        assert_eq!(back.container, ContainerFormat::Mkv);
        assert_eq!(back.video.width, 1920);
        assert_eq!(back.video.fps, 60);
    }

    #[test]
    fn container_extensions() {
        assert_eq!(ContainerFormat::Mkv.extension(), "mkv");
        assert_eq!(ContainerFormat::Mp4.extension(), "mp4");
    }

    #[test]
    fn stop_always_asks_in_m3() {
        assert!(stop_requires_confirmation());
    }
}
