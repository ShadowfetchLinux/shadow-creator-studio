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
    pub display_id: Option<String>,
    pub display_label: Option<String>,
    pub pip_corner: String,
    pub window_capture: bool,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            display_id: None,
            display_label: None,
            pip_corner: "bottom_right".into(),
            window_capture: false,
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
    pub mic_label: Option<String>,
    pub desktop_device: Option<String>,
    pub desktop_label: Option<String>,
    pub music_device: Option<String>,
    pub music_label: Option<String>,
    pub separate_tracks: bool,
    pub include_mixed: bool,
    pub include_mic: bool,
    pub include_desktop: bool,
    pub include_music: bool,
    pub mic_muted: bool,
    pub desktop_muted: bool,
    pub music_muted: bool,
    pub mic_volume: f32,
    pub desktop_volume: f32,
    pub music_volume: f32,
    pub monitor_enabled: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            processing_preset: AudioProcessingPreset::Natural,
            mic_device: None,
            mic_label: None,
            desktop_device: None,
            desktop_label: None,
            music_device: None,
            music_label: None,
            separate_tracks: true,
            include_mixed: true,
            include_mic: true,
            include_desktop: true,
            include_music: false,
            mic_muted: false,
            desktop_muted: false,
            music_muted: false,
            mic_volume: 1.0,
            desktop_volume: 1.0,
            music_volume: 1.0,
            monitor_enabled: false,
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
    Broadcast,
    QuietRoom,
    NoisyRoom,
}

impl AudioProcessingPreset {
    pub const ALL: [AudioProcessingPreset; 7] = [
        AudioProcessingPreset::Natural,
        AudioProcessingPreset::Podcast,
        AudioProcessingPreset::Broadcast,
        AudioProcessingPreset::QuietRoom,
        AudioProcessingPreset::NoisyRoom,
        AudioProcessingPreset::Voice,
        AudioProcessingPreset::Raw,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Natural => "Natural (light)",
            Self::Podcast => "Podcast",
            Self::Broadcast => "Broadcast",
            Self::QuietRoom => "Quiet Room",
            Self::NoisyRoom => "Noisy Room",
            Self::Voice => "Voice",
            Self::Raw => "Raw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    pub device: Option<String>,
    pub label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel_format: Option<String>,
    pub mirror_preview: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            device: None,
            label: None,
            width: 1920,
            height: 1080,
            fps: 30,
            pixel_format: None,
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
    /// Secret Service attribute name. Must never hold a raw stream key.
    pub stream_key_ref: Option<String>,
    pub enabled: bool,
    pub rtmps: bool,
    pub video_bitrate: String,
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub record_while_live: bool,
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self {
            platform: "youtube".into(),
            server_url: String::new(),
            stream_key_ref: None,
            enabled: false,
            rtmps: true,
            video_bitrate: "6000k".into(),
            reconnect_attempts: 8,
            reconnect_delay_ms: 2000,
            record_while_live: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeySettings {
    pub start_stop: String,
    pub pause: String,
    pub mute_mic: String,
    pub mute_desktop: String,
    pub marker: String,
    pub toggle_camera: String,
    pub toggle_teleprompter: String,
    /// Accidental stop still confirms unless the user turns this off.
    pub confirm_stop: bool,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            start_stop: "F9".into(),
            pause: "F10".into(),
            mute_mic: "F7".into(),
            mute_desktop: "Shift+F7".into(),
            marker: "F8".into(),
            toggle_camera: "F6".into(),
            toggle_teleprompter: "F5".into(),
            confirm_stop: true,
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

    #[test]
    fn persists_selected_devices_and_labels() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::in_dir(dir.path());
        let mut s = Settings::recommended();
        s.camera.device = Some("/dev/video0".into());
        s.camera.label = Some("USB Camera".into());
        s.camera.pixel_format = Some("mjpeg".into());
        s.camera.fps = 15;
        s.audio.mic_device = Some("alsa_input.usb-generic".into());
        s.audio.mic_label = Some("USB Microphone".into());
        s.audio.desktop_device = Some("alsa_output.speakers.monitor".into());
        s.audio.desktop_label = Some("Speakers (monitor)".into());
        s.video.display_id = Some("0".into());
        s.video.display_label = Some("HDMI-1 · 1920×1080".into());
        store.save(&s).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.camera.device.as_deref(), Some("/dev/video0"));
        assert_eq!(loaded.camera.label.as_deref(), Some("USB Camera"));
        assert_eq!(loaded.audio.mic_label.as_deref(), Some("USB Microphone"));
        assert_eq!(
            loaded.audio.desktop_device.as_deref(),
            Some("alsa_output.speakers.monitor")
        );
        assert_eq!(loaded.video.display_id.as_deref(), Some("0"));
        assert!(loaded.camera.mirror_preview);
    }
}
