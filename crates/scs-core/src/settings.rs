use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::modes::RecordingMode;
use crate::paths::{default_recordings_dir, settings_file};
use crate::quality::QualityPreset;
use crate::wizard::WizardState;
use crate::{migration, CoreResult};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: GeneralSettings,
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub camera: CameraSettings,
    pub recording: RecordingSettings,
    pub streaming: StreamingSettings,
    pub hotkeys: HotkeySettings,
    pub advanced: AdvancedSettings,
    pub wizard: WizardState,
    pub last_recording_mode: RecordingMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self::recommended()
    }
}

impl Settings {
    pub fn recommended() -> Self {
        Self {
            version: SETTINGS_VERSION,
            general: GeneralSettings::default(),
            video: VideoSettings::default(),
            audio: AudioSettings::default(),
            camera: CameraSettings::default(),
            recording: RecordingSettings::default(),
            streaming: StreamingSettings::default(),
            hotkeys: HotkeySettings::default(),
            advanced: AdvancedSettings::default(),
            wizard: WizardState::default(),
            last_recording_mode: RecordingMode::Creator,
        }
    }

    pub fn restore_recommended(&mut self) {
        let wizard = self.wizard.clone();
        *self = Self::recommended();
        self.wizard = wizard;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub force_dark: bool,
    pub show_wizard_from_menu: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            force_dark: true,
            show_wizard_from_menu: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoSettings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
        }
    }
}

impl VideoSettings {
    pub fn apply_quality(&mut self, quality: QualityPreset) {
        self.width = quality.width();
        self.height = quality.height();
        self.fps = quality.fps();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub processing_preset: AudioProcessingPreset,
    pub mic_device: Option<String>,
    pub desktop_device: Option<String>,
    pub separate_tracks: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            processing_preset: AudioProcessingPreset::Natural,
            mic_device: None,
            desktop_device: None,
            separate_tracks: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProcessingPreset {
    Raw,
    #[default]
    Natural,
    Voice,
    Podcast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    pub device: Option<String>,
    pub width: u32,
    pub height: u32,
    pub mirror_preview: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            device: None,
            width: 1920,
            height: 1080,
            mirror_preview: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    pub folder: PathBuf,
    pub container: crate::recording::ContainerFormat,
    pub remux_to_mp4: bool,
    /// Never true in recommended settings — MKV stays until MP4 is verified.
    pub delete_mkv_after_verified_mp4: bool,
    pub project_title: String,
    pub clip_title: String,
    pub quality: QualityPreset,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            folder: default_recordings_dir(),
            container: crate::recording::ContainerFormat::Mkv,
            remux_to_mp4: true,
            delete_mkv_after_verified_mp4: false,
            project_title: "YouTube".into(),
            clip_title: "Record".into(),
            quality: QualityPreset::Youtube1080p60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamingSettings {
    pub platform: String,
    pub server_url: String,
    /// Secret Service attribute name later. Must never hold a raw stream key.
    pub stream_key_ref: Option<String>,
    pub enabled: bool,
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self {
            platform: "youtube".into(),
            server_url: String::new(),
            stream_key_ref: None,
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeySettings {
    pub start_stop: String,
    pub pause: String,
    pub marker: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            start_stop: "F9".into(),
            pause: "F10".into(),
            marker: "F8".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub engine: EnginePreference,
    pub encoder: EncoderPreference,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            engine: EnginePreference::Auto,
            encoder: EncoderPreference::AutoNvenc,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnginePreference {
    #[default]
    Auto,
    Obs,
    Ffmpeg,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncoderPreference {
    #[default]
    AutoNvenc,
    NvencH264,
    NvencHevc,
    NvencAv1,
    SoftwareX264,
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    pub path: PathBuf,
}

impl SettingsStore {
    pub fn default_location() -> Self {
        Self {
            path: settings_file(),
        }
    }

    pub fn in_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            path: dir.as_ref().join("settings.json"),
        }
    }

    pub fn load(&self) -> CoreResult<Settings> {
        if !self.path.exists() {
            return Ok(Settings::recommended());
        }
        let raw = std::fs::read_to_string(&self.path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        migration::migrate(value)
    }

    pub fn save(&self, settings: &Settings) -> CoreResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommended_never_deletes_mkv() {
        let s = Settings::recommended();
        assert!(!s.recording.delete_mkv_after_verified_mp4);
        assert_eq!(s.recording.container, crate::recording::ContainerFormat::Mkv);
        assert_eq!(s.audio.processing_preset, AudioProcessingPreset::Natural);
        assert_eq!(s.last_recording_mode, RecordingMode::Creator);
    }

    #[test]
    fn json_round_trip_persists_mode_and_folder() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::in_dir(dir.path());
        let mut s = Settings::recommended();
        s.last_recording_mode = RecordingMode::Voice;
        s.recording.project_title = "Channel".into();
        s.recording.folder = dir.path().join("takes");
        store.save(&s).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.last_recording_mode, RecordingMode::Voice);
        assert_eq!(loaded.recording.project_title, "Channel");
        assert_eq!(loaded.recording.folder, dir.path().join("takes"));
        assert_eq!(loaded.version, SETTINGS_VERSION);
    }

    #[test]
    fn restore_recommended_keeps_wizard_completion() {
        let mut s = Settings::recommended();
        s.last_recording_mode = RecordingMode::Screen;
        s.wizard = WizardState::mark_completed();
        s.restore_recommended();
        assert_eq!(s.last_recording_mode, RecordingMode::Creator);
        assert!(s.wizard.completed);
    }

    #[test]
    fn missing_file_returns_recommended() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::in_dir(dir.path());
        let loaded = store.load().unwrap();
        assert_eq!(loaded, Settings::recommended());
    }
}
