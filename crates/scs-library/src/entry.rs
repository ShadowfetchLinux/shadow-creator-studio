use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::sidecar::Sidecar;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub path: PathBuf,
    pub title: String,
    pub modified: DateTime<Utc>,
    pub size_bytes: u64,
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub thumbnail_path: Option<PathBuf>,
}

impl LibraryEntry {
    pub fn from_path(path: &Path, meta: Option<&Metadata>, sidecar: Option<&Sidecar>) -> Self {
        let title = sidecar
            .and_then(|s| s.title.clone())
            .or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "Untitled".into());
        let modified = meta
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let size = meta.map(|m| m.len()).unwrap_or(0);
        Self {
            path: path.to_path_buf(),
            title,
            modified: DateTime::<Utc>::from(modified),
            size_bytes: sidecar.and_then(|s| s.size_bytes).unwrap_or(size),
            duration_secs: sidecar.and_then(|s| s.duration_secs),
            width: sidecar.and_then(|s| s.width),
            height: sidecar.and_then(|s| s.height),
            fps: sidecar.and_then(|s| s.fps),
            video_codec: sidecar.and_then(|s| s.video_codec.clone()),
            audio_codec: sidecar.and_then(|s| s.audio_codec.clone()),
            thumbnail_path: sidecar.and_then(|s| s.thumbnail_path.clone()),
        }
    }

    pub fn resolution_label(&self) -> String {
        match (self.width, self.height) {
            (Some(w), Some(h)) => format!("{w}×{h}"),
            _ => "Unknown resolution".into(),
        }
    }

    pub fn size_label(&self) -> String {
        let b = self.size_bytes as f64;
        if b >= 1_073_741_824.0 {
            format!("{:.1} GiB", b / 1_073_741_824.0)
        } else if b >= 1_048_576.0 {
            format!("{:.1} MiB", b / 1_048_576.0)
        } else {
            format!("{:.0} KiB", b / 1024.0)
        }
    }

    pub fn duration_label(&self) -> String {
        let Some(secs) = self.duration_secs else {
            return "Duration unknown".into();
        };
        let total = secs.max(0.0) as u64;
        format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_cover_unknowns() {
        let entry = LibraryEntry::from_path(Path::new("/tmp/demo.mkv"), None, None);
        assert_eq!(entry.title, "demo");
        assert_eq!(entry.resolution_label(), "Unknown resolution");
        assert_eq!(entry.duration_label(), "Duration unknown");
    }
}
