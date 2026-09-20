use serde::{Deserialize, Serialize};

use crate::display::DesktopKind;

/// A PipeWire desktop stream granted by xdg-desktop-portal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopStream {
    pub node_id: u32,
    pub width: u32,
    pub height: u32,
    pub kind: DesktopKind,
    pub restore_token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Default for PipCorner {
    fn default() -> Self {
        Self::BottomRight
    }
}

impl PipCorner {
    pub const ALL: [PipCorner; 4] = [
        PipCorner::BottomRight,
        PipCorner::BottomLeft,
        PipCorner::TopRight,
        PipCorner::TopLeft,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left",
            Self::TopRight => "Top right",
            Self::BottomLeft => "Bottom left",
            Self::BottomRight => "Bottom right",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "top_left" => Self::TopLeft,
            "top_right" => Self::TopRight,
            "bottom_left" => Self::BottomLeft,
            _ => Self::BottomRight,
        }
    }

    pub fn offset(self, canvas_w: u32, canvas_h: u32, pip_w: u32, pip_h: u32, margin: u32) -> (u32, u32) {
        let m = margin;
        match self {
            Self::TopLeft => (m, m),
            Self::TopRight => (canvas_w.saturating_sub(pip_w + m), m),
            Self::BottomLeft => (m, canvas_h.saturating_sub(pip_h + m)),
            Self::BottomRight => (
                canvas_w.saturating_sub(pip_w + m),
                canvas_h.saturating_sub(pip_h + m),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipSpec {
    pub corner: PipCorner,
    pub width: u32,
    pub height: u32,
}

impl Default for PipSpec {
    fn default() -> Self {
        Self {
            corner: PipCorner::BottomRight,
            width: 480,
            height: 270,
        }
    }
}

/// Bitmask from org.freedesktop.portal.ScreenCast AvailableSourceTypes.
pub const SOURCE_MONITOR: u32 = 1;
pub const SOURCE_WINDOW: u32 = 2;

pub fn parse_stream_props(node_id: u32, width: Option<u32>, height: Option<u32>, source_type: Option<u32>) -> DesktopStream {
    let kind = match source_type.unwrap_or(SOURCE_MONITOR) {
        SOURCE_WINDOW => DesktopKind::Window,
        _ => DesktopKind::Display,
    };
    DesktopStream {
        node_id,
        width: width.unwrap_or(1920).max(1),
        height: height.unwrap_or(1080).max(1),
        kind,
        restore_token: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pip_bottom_right() {
        let (x, y) = PipCorner::BottomRight.offset(1920, 1080, 480, 270, 24);
        assert_eq!((x, y), (1416, 786));
    }

    #[test]
    fn parse_window_stream() {
        let s = parse_stream_props(42, Some(1280), Some(720), Some(SOURCE_WINDOW));
        assert_eq!(s.node_id, 42);
        assert_eq!(s.kind, DesktopKind::Window);
    }
}
