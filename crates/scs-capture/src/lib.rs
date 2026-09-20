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
