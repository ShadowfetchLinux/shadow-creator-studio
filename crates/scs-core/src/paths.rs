use std::path::PathBuf;

use crate::APP_ID;

/// `$XDG_CONFIG_HOME/com.shadowfetch.creatorstudio`
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

/// `$XDG_VIDEOS_DIR/Shadow Creator Studio` (falls back to `$HOME`).
pub fn default_recordings_dir() -> PathBuf {
    dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Shadow Creator Studio")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_uses_app_id() {
        let dir = config_dir();
        assert!(
            dir.ends_with(APP_ID),
            "expected config dir to end with {APP_ID}, got {}",
            dir.display()
        );
    }

    #[test]
    fn recordings_dir_has_studio_name() {
        assert!(default_recordings_dir().ends_with("Shadow Creator Studio"));
    }
}
