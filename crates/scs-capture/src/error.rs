use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    NotFound { what: String },
    Busy { what: String },
    Permission { what: String },
    Failed { what: String, detail: String },
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.human_message())
    }
}

impl std::error::Error for CaptureError {}

impl CaptureError {
    pub fn human_message(&self) -> String {
        match self {
            Self::NotFound { what } => format!("No {what} is available."),
            Self::Busy { what } => format!("{what} is busy in another application."),
            Self::Permission { what } => {
                format!("Cannot open {what} — permission denied.")
            }
            Self::Failed { what, detail } => format!("{what}: {detail}"),
        }
    }
}

pub fn map_tool_text(what: &str, text: &str) -> CaptureError {
    let lower = text.to_ascii_lowercase();
    if lower.contains("permission denied") {
        CaptureError::Permission {
            what: what.into(),
        }
    } else if lower.contains("busy") {
        CaptureError::Busy {
            what: what.into(),
        }
    } else if lower.contains("no such file") || lower.contains("not found") {
        CaptureError::NotFound {
            what: what.into(),
        }
    } else {
        let detail = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("failed")
            .chars()
            .take(200)
            .collect();
        CaptureError::Failed {
            what: what.into(),
            detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ffmpeg_and_v4l_errors() {
        assert!(matches!(
            map_tool_text("camera", "Permission denied"),
            CaptureError::Permission { .. }
        ));
        assert!(matches!(
            map_tool_text("camera", "Device or resource busy"),
            CaptureError::Busy { .. }
        ));
        assert!(map_tool_text("camera", "Immediate exit requested")
            .human_message()
            .contains("camera"));
    }
}
