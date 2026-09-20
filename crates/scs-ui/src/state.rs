use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Instant;

use scs_core::{
    MarkerFile, RecordingMode, Settings, SettingsStore, APP_NAME,
};
use scs_teleprompter::TeleprompterScript;
use scs_encoder::{capabilities_from_encoder_list, EncoderCapabilities};
use scs_system::SystemMonitor;

pub struct StudioState {
    pub settings: RefCell<Settings>,
    pub store: SettingsStore,
    pub monitor: RefCell<SystemMonitor>,
    pub encoder_status: RefCell<String>,
    pub caps: EncoderCapabilities,
    pub recording: RefCell<RecordingUi>,
    pub teleprompter: RefCell<TeleprompterScript>,
    pub markers: RefCell<MarkerFile>,
    pub camera_preview: std::cell::Cell<bool>,
    pub muxers: RefCell<String>,
    pub desktop_share: RefCell<Option<crate::live::portal::SharedDesktop>>,
    pub desktop_note: RefCell<Option<String>>,
}

#[derive(Debug, Default, Clone)]
pub struct RecordingUi {
    pub active: bool,
    pub started: Option<Instant>,
    pub path: Option<PathBuf>,
    pub encoder: String,
    pub warning: Option<String>,
    pub dropped: Option<u64>,
    pub last_message: Option<String>,
    pub last_marker: Option<String>,
}

impl StudioState {
    pub fn load() -> Self {
        let store = SettingsStore::default_location();
        let settings = store.load().unwrap_or_else(|_| Settings::recommended());
        let disk = settings.recording.folder.clone();
        let (caps, encoder_status) = probe_encoders();
        Self {
            settings: RefCell::new(settings),
            store,
            monitor: RefCell::new(SystemMonitor::new(disk)),
            encoder_status: RefCell::new(encoder_status),
            caps,
            recording: RefCell::new(RecordingUi::default()),
            teleprompter: RefCell::new(TeleprompterScript::default()),
            markers: RefCell::new(MarkerFile::new("idle")),
            camera_preview: std::cell::Cell::new(true),
            muxers: RefCell::new(probe_muxers()),
            desktop_share: RefCell::new(None),
            desktop_note: RefCell::new(None),
        }
    }

    pub fn persist(&self) -> Result<(), String> {
        self.store
            .save(&self.settings.borrow())
            .map_err(|err| err.to_string())
    }

    pub fn select_mode(&self, mode: RecordingMode) -> Result<(), String> {
        if self.recording.borrow().active {
            return Err("Finish the current take before changing mode.".into());
        }
        self.settings.borrow_mut().last_recording_mode = mode;
        self.persist()
    }

    pub fn restore_recommended(&self) -> Result<(), String> {
        self.settings.borrow_mut().restore_recommended();
        self.sync_monitor_path();
        self.persist()
    }

    pub fn set_folder(&self, folder: PathBuf) -> Result<(), String> {
        let _ = std::fs::create_dir_all(&folder);
        self.settings.borrow_mut().recording.folder = folder;
        self.sync_monitor_path();
        self.persist()
    }

    pub fn complete_wizard(&self) -> Result<(), String> {
        self.settings.borrow_mut().wizard = scs_core::WizardState::mark_completed();
        self.persist()
    }

    pub fn wizard_completed(&self) -> bool {
        self.settings.borrow().wizard.completed
    }

    fn sync_monitor_path(&self) {
        let folder = self.settings.borrow().recording.folder.clone();
        self.monitor.borrow_mut().set_disk_path(folder);
    }
}

fn probe_encoders() -> (EncoderCapabilities, String) {
    let output = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let caps = capabilities_from_encoder_list(&text);
            let status = if caps.h264_nvenc.is_available() {
                "h264_nvenc ready · idle"
            } else if caps.hevc_nvenc.is_available() {
                "hevc_nvenc ready · idle"
            } else if caps.av1_nvenc.is_available() {
                "av1_nvenc ready · idle"
            } else if caps.libx264.is_available() {
                "libx264 ready · idle"
            } else {
                "Unavailable — no known encoder listed"
            };
            (caps, status.into())
        }
        Err(_) => (
            EncoderCapabilities::unknown(),
            "Unavailable — ffmpeg not available".into(),
        ),
    }
}

fn probe_muxers() -> String {
    std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-muxers"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

pub fn app_subtitle() -> String {
    format!("{APP_NAME} · 0.1")
}
