use std::cell::RefCell;
use std::path::PathBuf;

use scs_core::{
    RecordingMode, Settings, SettingsStore, APP_NAME,
};
use scs_system::SystemMonitor;

pub struct StudioState {
    pub settings: RefCell<Settings>,
    pub store: SettingsStore,
    pub monitor: RefCell<SystemMonitor>,
    pub encoder_status: RefCell<String>,
}

impl StudioState {
    pub fn load() -> Self {
        let store = SettingsStore::default_location();
        let settings = store.load().unwrap_or_else(|_| Settings::recommended());
        let disk = settings.recording.folder.clone();
        let encoder_status = probe_encoder_status();
        Self {
            settings: RefCell::new(settings),
            store,
            monitor: RefCell::new(SystemMonitor::new(disk)),
            encoder_status: RefCell::new(encoder_status),
        }
    }

    pub fn persist(&self) -> Result<(), String> {
        self.store
            .save(&self.settings.borrow())
            .map_err(|err| err.to_string())
    }

    pub fn select_mode(&self, mode: RecordingMode) -> Result<(), String> {
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

fn probe_encoder_status() -> String {
    let output = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let caps = scs_encoder::capabilities_from_encoder_list(&text);
            if caps.h264_nvenc.is_available() {
                "NVENC ready · idle".into()
            } else if caps.libx264.is_available() {
                "libx264 ready · idle".into()
            } else {
                "Unavailable — no known encoder listed".into()
            }
        }
        Err(_) => "Unavailable — ffmpeg not available".into(),
    }
}

pub fn format_metric(value: Option<String>) -> (String, bool) {
    match value {
        Some(v) => (v, true),
        None => ("—".into(), false),
    }
}

pub fn app_subtitle() -> String {
    format!("{APP_NAME} · Milestone 1")
}
