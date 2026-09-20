use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    Mixed,
    Microphone,
    Desktop,
    Music,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBus {
    pub role: TrackRole,
    pub enabled: bool,
    pub muted: bool,
    pub volume: f32,
    pub source: Option<String>,
    /// Emit this bus as its own recorded stream (in addition to the mix).
    pub write_track: bool,
}

impl AudioBus {
    pub fn new(role: TrackRole) -> Self {
        Self {
            role,
            enabled: !matches!(role, TrackRole::Music),
            muted: false,
            volume: 1.0,
            source: None,
            write_track: !matches!(role, TrackRole::Music),
        }
    }

    pub fn effective_volume(&self) -> f32 {
        if !self.enabled || self.muted {
            0.0
        } else {
            self.volume.clamp(0.0, 2.5)
        }
    }

    pub fn audible(&self) -> bool {
        self.enabled && !self.muted && self.volume > 0.001
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackLayout {
    pub mixed: AudioBus,
    pub microphone: AudioBus,
    pub desktop: AudioBus,
    pub music: AudioBus,
}

impl Default for TrackLayout {
    fn default() -> Self {
        Self {
            mixed: AudioBus::new(TrackRole::Mixed),
            microphone: AudioBus::new(TrackRole::Microphone),
            desktop: AudioBus::new(TrackRole::Desktop),
            music: AudioBus::new(TrackRole::Music),
        }
    }
}

impl TrackLayout {
    pub fn with_sources(mut self, mic: Option<String>, desktop: Option<String>, music: Option<String>) -> Self {
        self.microphone.source = mic;
        self.desktop.source = desktop;
        self.music.source = music;
        if self.desktop.source.is_none() {
            self.desktop.enabled = false;
        }
        if self.music.source.is_none() {
            self.music.enabled = false;
        }
        self
    }

    pub fn enabled_roles(&self) -> Vec<TrackRole> {
        let mut roles = Vec::new();
        if self.mixed.enabled && self.mixed.write_track {
            roles.push(TrackRole::Mixed);
        }
        if self.microphone.audible() && self.microphone.write_track {
            roles.push(TrackRole::Microphone);
        }
        if self.desktop.audible() && self.desktop.source.is_some() && self.desktop.write_track {
            roles.push(TrackRole::Desktop);
        }
        if self.music.audible() && self.music.source.is_some() && self.music.write_track {
            roles.push(TrackRole::Music);
        }
        roles
    }

    pub fn from_settings(audio: &scs_core::settings::AudioSettings) -> Self {
        let mut layout = Self::default().with_sources(
            audio.mic_device.clone(),
            audio.desktop_device.clone(),
            audio.music_device.clone(),
        );
        layout.microphone.volume = audio.mic_volume;
        layout.desktop.volume = audio.desktop_volume;
        layout.music.volume = audio.music_volume;
        layout.microphone.muted = audio.mic_muted;
        layout.desktop.muted = audio.desktop_muted;
        layout.music.muted = audio.music_muted;
        layout.mixed.enabled = audio.include_mixed || !audio.separate_tracks;
        layout.mixed.write_track = audio.include_mixed || !audio.separate_tracks;
        layout.microphone.enabled = audio.include_mic || !audio.separate_tracks;
        layout.microphone.write_track = audio.separate_tracks && audio.include_mic;
        layout.desktop.enabled = audio.include_desktop && audio.desktop_device.is_some();
        layout.desktop.write_track = audio.separate_tracks && audio.include_desktop;
        layout.music.enabled = audio.include_music && audio.music_device.is_some();
        layout.music.write_track = audio.separate_tracks && audio.include_music;
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_has_mic_and_mixed() {
        let layout = TrackLayout::default();
        assert!(layout.mixed.enabled);
        assert!(layout.microphone.enabled);
        assert!(!layout.music.enabled);
        assert_eq!(layout.microphone.effective_volume(), 1.0);
        let settings = scs_core::settings::AudioSettings::default();
        let from = TrackLayout::from_settings(&settings);
        assert!(from.mixed.write_track);
        assert!(from.microphone.write_track);
        assert!(!from.music.write_track);
    }

    #[test]
    fn mute_and_volume() {
        let mut bus = AudioBus::new(TrackRole::Microphone);
        bus.volume = 1.5;
        assert!((bus.effective_volume() - 1.5).abs() < f32::EPSILON);
        bus.muted = true;
        assert_eq!(bus.effective_volume(), 0.0);
        assert!(!bus.audible());
    }

    #[test]
    fn settings_layout_respects_mute() {
        let mut audio = scs_core::settings::AudioSettings::default();
        audio.mic_muted = true;
        audio.include_desktop = true;
        audio.desktop_device = Some("desk".into());
        let layout = TrackLayout::from_settings(&audio);
        assert!(!layout.microphone.audible());
        assert!(layout.desktop.enabled);
        assert!(!layout.music.enabled);
    }
}
