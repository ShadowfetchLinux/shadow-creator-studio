use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerKind {
    Chapter,
    Highlight,
    Mistake,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub id: String,
    pub timestamp_ms: u64,
    pub label: String,
    pub kind: MarkerKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerFile {
    pub version: u32,
    pub recording_id: String,
    pub markers: Vec<Marker>,
}

impl MarkerFile {
    pub fn new(recording_id: impl Into<String>) -> Self {
        Self {
            version: 1,
            recording_id: recording_id.into(),
            markers: Vec::new(),
        }
    }

    pub fn add(&mut self, timestamp_ms: u64, label: impl Into<String>) -> &Marker {
        let n = self.markers.len() + 1;
        self.markers.push(Marker {
            id: format!("m{n}"),
            timestamp_ms,
            label: label.into(),
            kind: MarkerKind::Chapter,
        });
        self.markers.last().expect("just pushed")
    }

    pub fn path_for(media: impl AsRef<std::path::Path>) -> std::path::PathBuf {
        let media = media.as_ref();
        let stem = media
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "take".into());
        media.with_file_name(format!("{stem}.markers.json"))
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path.as_ref(), json).map_err(|e| e.to_string())
    }

    pub fn chapter_vtt(&self) -> String {
        let mut out = String::from("WEBVTT\n\n");
        for (i, marker) in self.markers.iter().enumerate() {
            let start = format_ts(marker.timestamp_ms);
            let end = self
                .markers
                .get(i + 1)
                .map(|m| format_ts(m.timestamp_ms))
                .unwrap_or_else(|| format_ts(marker.timestamp_ms + 2_000));
            out.push_str(&format!("{start} --> {end}\n{}\n\n", marker.label));
        }
        out
    }
}

fn format_ts(ms: u64) -> String {
    let s = ms / 1000;
    let rem = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", s / 3600, (s % 3600) / 60, s % 60, rem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_file_round_trip() {
        let mut file = MarkerFile::new("rec-1");
        file.markers.push(Marker {
            id: "m1".into(),
            timestamp_ms: 12_500,
            label: "Intro done".into(),
            kind: MarkerKind::Chapter,
        });
        let json = serde_json::to_string(&file).unwrap();
        let back: MarkerFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, 1);
        assert_eq!(back.markers[0].kind, MarkerKind::Chapter);
        assert_eq!(back.markers[0].timestamp_ms, 12_500);
        assert!(json.contains("\"chapter\""));
        file.add(20_000, "Hook");
        assert_eq!(file.markers.len(), 2);
        assert!(file.chapter_vtt().contains("00:00:12.500"));
        assert!(MarkerFile::path_for("/tmp/a.mkv")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("markers"));
    }
}
