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
    }
}
