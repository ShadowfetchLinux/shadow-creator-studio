use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleprompterScript {
    pub title: String,
    pub body: String,
    pub scroll_speed: f32,
}

impl Default for TeleprompterScript {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            scroll_speed: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_script_is_empty() {
        let script = TeleprompterScript::default();
        assert!(script.body.is_empty());
        assert_eq!(script.scroll_speed, 1.0);
    }
}
