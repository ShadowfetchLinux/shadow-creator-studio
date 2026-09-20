use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    #[default]
    #[serde(rename = "youtube_1080p60")]
    Youtube1080p60,
    #[serde(rename = "youtube_1080p30")]
    Youtube1080p30,
    #[serde(rename = "youtube_1440p60")]
    Youtube1440p60,
    #[serde(rename = "youtube_4k30")]
    Youtube4k30,
    High,
    Balanced,
    Custom,
}

impl QualityPreset {
    pub const ALL: [QualityPreset; 7] = [
        QualityPreset::Youtube1080p60,
        QualityPreset::Youtube1080p30,
        QualityPreset::Youtube1440p60,
        QualityPreset::Youtube4k30,
        QualityPreset::High,
        QualityPreset::Balanced,
        QualityPreset::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Youtube1080p60 => "YouTube 1080p60",
            Self::Youtube1080p30 => "YouTube 1080p30",
            Self::Youtube1440p60 => "YouTube 1440p60",
            Self::Youtube4k30 => "YouTube 4K30",
            Self::High => "High",
            Self::Balanced => "Balanced",
            Self::Custom => "Custom",
        }
    }

    pub fn width(self) -> u32 {
        match self {
            Self::Youtube4k30 => 3840,
            Self::Youtube1440p60 => 2560,
            _ => 1920,
        }
    }

    pub fn height(self) -> u32 {
        match self {
            Self::Youtube4k30 => 2160,
            Self::Youtube1440p60 => 1440,
            _ => 1080,
        }
    }

    pub fn fps(self) -> u32 {
        match self {
            Self::Youtube1080p30 | Self::Youtube4k30 => 30,
            _ => 60,
        }
    }

    /// Conservative total bitrate used only for disk-time estimates.
    pub fn estimated_bitrate_bps(self) -> u64 {
        let video = match self {
            Self::Youtube1080p30 => 12_000_000,
            Self::Youtube1080p60 => 16_000_000,
            Self::Youtube1440p60 => 24_000_000,
            Self::Youtube4k30 => 35_000_000,
            Self::High => 20_000_000,
            Self::Balanced => 12_000_000,
            Self::Custom => 16_000_000,
        };
        let audio = 320_000 * 2; // mic + desktop planning tracks
        video + audio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_1080p60_geometry() {
        let q = QualityPreset::Youtube1080p60;
        assert_eq!(q.width(), 1920);
        assert_eq!(q.height(), 1080);
        assert_eq!(q.fps(), 60);
        assert!(q.estimated_bitrate_bps() > 16_000_000);
    }
}
