use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplaySource {
    pub id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub primary: bool,
}

impl DisplaySource {
    pub fn geometry_label(&self) -> String {
        let primary = if self.primary { " · primary" } else { "" };
        format!(
            "{} · {}×{}{primary}",
            self.label, self.width, self.height
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopKind {
    Display,
    Window,
    Region,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopOption {
    pub kind: DesktopKind,
    pub available: bool,
    pub reason: Option<String>,
}

impl DesktopOption {
    pub fn window_unavailable() -> Self {
        Self {
            kind: DesktopKind::Window,
            available: false,
            reason: Some("Window capture arrives in a later milestone.".into()),
        }
    }

    pub fn region_unavailable() -> Self {
        Self {
            kind: DesktopKind::Region,
            available: false,
            reason: Some("Region capture arrives in a later milestone.".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_label_marks_primary() {
        let display = DisplaySource {
            id: "0".into(),
            label: "HDMI-1".into(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            primary: true,
        };
        assert!(display.geometry_label().contains("primary"));
        assert!(display.geometry_label().contains("1920×1080"));
    }
}
