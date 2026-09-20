//! Capture sources: cameras, displays, and composite layouts.

mod desktop;
mod display;
mod error;
mod formats;
mod inventory;
mod v4l2;

pub use desktop::{parse_stream_props, DesktopStream, PipCorner, PipSpec, SOURCE_MONITOR, SOURCE_WINDOW};
pub use display::{DesktopKind, DesktopOption, DisplaySource};
pub use error::CaptureError;
pub use formats::parse_v4l2_list_formats;
pub use inventory::DeviceInventory;
pub use v4l2::{enumerate_cameras, enumerate_sysfs, prefer_format, CameraDevice, CameraFormat};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CaptureSource {
    Screen { connector: Option<String> },
    Window { title: Option<String> },
    Camera { device: Option<String> },
    Composite { screen: Box<CaptureSource>, camera: Box<CaptureSource> },
}

impl CaptureSource {
    pub fn creator_default() -> Self {
        Self::Composite {
            screen: Box::new(Self::Screen { connector: None }),
            camera: Box::new(Self::Camera { device: None }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_layout_serde() {
        let src = CaptureSource::creator_default();
        let json = serde_json::to_string(&src).unwrap();
        let back: CaptureSource = serde_json::from_str(&json).unwrap();
        assert_eq!(src, back);
        assert!(json.contains("composite"));
    }
}
