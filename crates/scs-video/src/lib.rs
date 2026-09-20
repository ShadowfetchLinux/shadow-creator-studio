use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const HD1080: Self = Self {
        width: 1920,
        height: 1080,
    };

    pub fn label(self) -> String {
        format!("{}×{}", self.width, self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorRange {
    Limited,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFormat {
    pub resolution: Resolution,
    pub fps: u32,
    pub color_range: ColorRange,
}

impl VideoFormat {
    pub fn youtube_1080p60() -> Self {
        Self {
            resolution: Resolution::HD1080,
            fps: 60,
            color_range: ColorRange::Limited,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_and_default() {
        let fmt = VideoFormat::youtube_1080p60();
        assert_eq!(fmt.resolution.label(), "1920×1080");
        assert_eq!(fmt.fps, 60);
    }
}
