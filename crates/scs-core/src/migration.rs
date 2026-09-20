use std::path::PathBuf;

use serde_json::Value;

use crate::quality::QualityPreset;
use crate::settings::{Settings, SETTINGS_VERSION};
use crate::{CoreError, CoreResult};

/// Upgrade on-disk JSON to the current [`Settings`] shape.
pub fn migrate(value: Value) -> CoreResult<Settings> {
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(0);
    match version {
        0 => migrate_v0_to_v1(value),
        1 => {
            let mut settings: Settings = serde_json::from_value(value)?;
            settings.version = SETTINGS_VERSION;
            Ok(settings)
        }
        other => Err(CoreError::UnsupportedSettingsVersion(other)),
    }
}

fn migrate_v0_to_v1(value: Value) -> CoreResult<Settings> {
    let mut settings = Settings::recommended();
    let Some(map) = value.as_object() else {
        settings.version = SETTINGS_VERSION;
        return Ok(settings);
    };

    if let Some(mode) = map.get("last_recording_mode") {
        if let Ok(parsed) = serde_json::from_value(mode.clone()) {
            settings.last_recording_mode = parsed;
        }
    }
    if let Some(folder) = map.get("recording_folder").and_then(Value::as_str) {
        settings.recording.folder = PathBuf::from(folder);
    }
    if let Some(quality) = map.get("quality") {
        if let Ok(parsed) = serde_json::from_value::<QualityPreset>(quality.clone()) {
            settings.recording.quality = parsed;
            settings.video.apply_quality(parsed);
        }
    }
    if let Some(recording) = map.get("recording") {
        if let Ok(parsed) = serde_json::from_value(recording.clone()) {
            settings.recording = parsed;
            settings.video.apply_quality(settings.recording.quality);
        }
    }
    if let Some(wizard) = map.get("wizard") {
        if let Ok(parsed) = serde_json::from_value(wizard.clone()) {
            settings.wizard = parsed;
        }
    }

    settings.version = SETTINGS_VERSION;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::RecordingMode;
    use serde_json::json;

    #[test]
    fn empty_object_becomes_v1_recommended() {
        let settings = migrate(json!({})).unwrap();
        assert_eq!(settings.version, 1);
        assert_eq!(settings.last_recording_mode, RecordingMode::Creator);
    }

    #[test]
    fn overlays_legacy_folder_quality_and_mode() {
        let settings = migrate(json!({
            "recording_folder": "/tmp/shadow-takes",
            "quality": "youtube_1080p30",
            "last_recording_mode": "voice"
        }))
        .unwrap();
        assert_eq!(settings.version, 1);
        assert_eq!(
            settings.recording.folder,
            PathBuf::from("/tmp/shadow-takes")
        );
        assert_eq!(settings.recording.quality, QualityPreset::Youtube1080p30);
        assert_eq!(settings.video.fps, 30);
        assert_eq!(settings.last_recording_mode, RecordingMode::Voice);
    }

    #[test]
    fn current_document_deserializes() {
        let original = Settings::recommended();
        let value = serde_json::to_value(&original).unwrap();
        let migrated = migrate(value).unwrap();
        assert_eq!(migrated.version, 1);
        assert_eq!(migrated.recording.project_title, "YouTube");
    }

    #[test]
    fn future_version_is_rejected() {
        let err = migrate(json!({"version": 99})).unwrap_err();
        match err {
            CoreError::UnsupportedSettingsVersion(99) => {}
            other => panic!("unexpected {other}"),
        }
    }
}
