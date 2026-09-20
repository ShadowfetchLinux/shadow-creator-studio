use serde::{Deserialize, Serialize};

/// Large Record-page tiles. Persisted in settings in M1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    Camera,
    Screen,
    Presentation,
    Voice,
    #[default]
    Creator,
    Custom,
}

impl RecordingMode {
    pub const ALL: [RecordingMode; 6] = [
        RecordingMode::Camera,
        RecordingMode::Screen,
        RecordingMode::Presentation,
        RecordingMode::Voice,
        RecordingMode::Creator,
        RecordingMode::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
            Self::Screen => "Screen",
            Self::Presentation => "Presentation",
            Self::Voice => "Voice",
            Self::Creator => "Creator",
            Self::Custom => "Custom",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Camera => "Talking head",
            Self::Screen => "Full desktop",
            Self::Presentation => "Slides + voice",
            Self::Voice => "Mic only",
            Self::Creator => "Camera + mic",
            Self::Custom => "Manual layout",
        }
    }

    /// Modes that actually write a file in Milestone 3.
    pub fn records_locally(self) -> bool {
        matches!(
            self,
            Self::Camera | Self::Voice | Self::Creator | Self::Custom
        )
    }

    pub fn unavailable_reason(self) -> Option<&'static str> {
        match self {
            Self::Screen | Self::Presentation => Some(
                "Desktop capture is not recorded yet. A portal/PipeWire grab would be required; this session will not fake a screen take.",
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_snake_case() {
        let json = serde_json::to_string(&RecordingMode::Creator).unwrap();
        assert_eq!(json, "\"creator\"");
        let back: RecordingMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, RecordingMode::Creator);
    }

    #[test]
    fn m3_modes_are_honest() {
        assert!(RecordingMode::Camera.records_locally());
        assert!(RecordingMode::Voice.records_locally());
        assert!(RecordingMode::Screen.unavailable_reason().is_some());
    }
}
