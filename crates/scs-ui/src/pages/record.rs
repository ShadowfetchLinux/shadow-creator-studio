use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use gtk::gdk::prelude::MonitorExt;
use gtk::gio::prelude::ListModelExt;
use gtk::prelude::*;
use scs_capture::{DeviceInventory, DisplaySource};
use scs_core::RecordingMode;
use scs_system::SystemSnapshot;

use crate::live::meters::{self as meter_live, MeterEvent};
use crate::live::preview::{self as preview_live, PreviewEvent};
use crate::state::StudioState;
use crate::widgets::{dashboard, meters, mode_tiles, preview, sources, status_strip};

pub struct RecordPage {
    pub root: gtk::ScrolledWindow,
    status: status_strip::StatusStrip,
    dashboard: dashboard::Dashboard,
    preview: preview::PreviewView,
    meters: meters::MeterPair,
    sources: sources::SourceSelector,
    live: RefCell<LiveSessions>,
    inventory: RefCell<DeviceInventory>,
    displays: RefCell<Vec<DisplaySource>>,
}

struct LiveSessions {
    preview_stop: Option<Arc<AtomicBool>>,
    mic_stop: Option<Arc<AtomicBool>>,
    desk_stop: Option<Arc<AtomicBool>>,
    preview_rx: Option<Receiver<PreviewEvent>>,
    mic_rx: Option<Receiver<MeterEvent>>,
    desk_rx: Option<Receiver<MeterEvent>>,
    cam_key: Option<(String, bool)>,
    mic_key: Option<String>,
    desk_key: Option<String>,
}

impl LiveSessions {
    fn new() -> Self {
        Self {
            preview_stop: None,
            mic_stop: None,
            desk_stop: None,
            preview_rx: None,
            mic_rx: None,
            desk_rx: None,
            cam_key: None,
            mic_key: None,
            desk_key: None,
        }
    }
}

impl RecordPage {
    pub fn new(state: &Rc<StudioState>) -> Self {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
        column.set_margin_top(20);
        column.set_margin_bottom(24);
        column.set_margin_start(24);
        column.set_margin_end(24);

        let heading = gtk::Label::new(Some("Record"));
        heading.add_css_class("title-1");
        heading.set_halign(gtk::Align::Start);

        let modes = mode_tiles::build(state);
        let sources = sources::SourceSelector::build(state);
        let preview = preview::PreviewView::build();
        let meters = meters::build();
        let status = status_strip::StatusStrip::build();

        let mirror = gtk::CheckButton::with_label("Mirror camera preview");
        mirror.set_active(state.settings.borrow().camera.mirror_preview);
        let state_m = Rc::clone(state);
        mirror.connect_toggled(move |btn| {
            state_m.settings.borrow_mut().camera.mirror_preview = btn.is_active();
            let _ = state_m.persist();
        });

        let rec = gtk::Button::with_label("START RECORDING");
        rec.add_css_class("scs-rec-button");
        rec.add_css_class("destructive-action");
        rec.set_sensitive(false);
        rec.set_halign(gtk::Align::Center);
        rec.set_tooltip_text(Some("Available in a later milestone (M3)"));

        let rec_note = gtk::Label::new(Some(
            "Available in a later milestone — Milestone 2 previews only. Nothing is recorded.",
        ));
        rec_note.add_css_class("scs-unavailable");
        rec_note.set_halign(gtk::Align::Center);

        let live_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        live_card.add_css_class("scs-card");
        let live_title = gtk::Label::new(Some("GO LIVE"));
        live_title.add_css_class("heading");
        live_title.set_halign(gtk::Align::Start);
        let live_btn = gtk::Button::with_label("GO LIVE");
        live_btn.add_css_class("scs-live-button");
        live_btn.set_sensitive(false);
        live_btn.set_halign(gtk::Align::Start);
        live_btn.set_tooltip_text(Some("Available in a later milestone (M8)"));
        let live_note = gtk::Label::new(Some(
            "YouTube Live is designed here for later expansion. Unavailable in Milestone 2.",
        ));
        live_note.add_css_class("scs-unavailable");
        live_note.set_halign(gtk::Align::Start);
        live_note.set_wrap(true);
        live_card.append(&live_title);
        live_card.append(&live_btn);
        live_card.append(&live_note);

        let dash_label = gtk::Label::new(Some("System"));
        dash_label.add_css_class("heading");
        dash_label.set_halign(gtk::Align::Start);
        let dashboard = dashboard::Dashboard::build();

        column.append(&heading);
        column.append(&modes);
        column.append(&sources.root);
        column.append(&preview.root);
        column.append(&mirror);
        column.append(&meters.root);
        column.append(&status.root);
        column.append(&rec);
        column.append(&rec_note);
        column.append(&live_card);
        column.append(&dash_label);
        column.append(&dashboard.root);

        let root = gtk::ScrolledWindow::new();
        root.set_child(Some(&column));
        root.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        Self {
            root,
            status,
            dashboard,
            preview,
            meters,
            sources,
            live: RefCell::new(LiveSessions::new()),
            inventory: RefCell::new(DeviceInventory::default()),
            displays: RefCell::new(Vec::new()),
        }
    }

    pub fn apply_inventory(&self, inventory: DeviceInventory, state: &Rc<StudioState>) {
        self.sources
            .set_cameras(&inventory.cameras, state);
        self.sources
            .set_audio(&inventory.microphones, &inventory.desktop_audio, state);
        *self.inventory.borrow_mut() = inventory;
        self.apply_displays(state);
        persist_defaults(state, &self.inventory.borrow(), &self.displays.borrow());
        self.sync_sessions(state);
    }

    pub fn apply_displays(&self, state: &Rc<StudioState>) {
        let displays = gdk_displays();
        self.sources.set_displays(&displays, state);
        *self.displays.borrow_mut() = displays;
    }

    pub fn refresh(&self, state: &Rc<StudioState>, snap: &SystemSnapshot) {
        self.sync_sessions(state);
        self.pump(state, snap);
    }

    pub fn pump(&self, state: &StudioState, snap: &SystemSnapshot) {
        self.status.refresh(state, snap);
        self.dashboard.refresh(state, snap);
        self.drain_preview();
        self.drain_meters(state);
    }

    pub fn sync_sessions(&self, state: &Rc<StudioState>) {
        let mode = state.settings.borrow().last_recording_mode;
        let inventory = self.inventory.borrow();
        let settings = state.settings.borrow();
        let want_cam = matches!(
            mode,
            RecordingMode::Camera | RecordingMode::Creator | RecordingMode::Custom
        );
        let camera = settings.camera.device.clone().and_then(|path| {
            inventory
                .cameras
                .iter()
                .find(|c| c.path == path)
                .cloned()
        });
        let mirror = settings.camera.mirror_preview;
        let mic = settings.audio.mic_device.clone();
        let desk = settings.audio.desktop_device.clone();
        let mic_label = settings
            .audio
            .mic_label
            .clone()
            .unwrap_or_else(|| "Microphone".into());
        let desk_label = settings
            .audio
            .desktop_label
            .clone()
            .unwrap_or_else(|| "Desktop audio".into());
        let display_label = settings
            .video
            .display_label
            .clone()
            .unwrap_or_else(|| "No display selected".into());
        drop(settings);

        if !want_cam {
            self.stop_preview();
            match mode {
                RecordingMode::Voice => self.preview.show_message(
                    "Voice mode",
                    "Camera preview is off. Meters stay live so you can set level.",
                ),
                RecordingMode::Screen | RecordingMode::Presentation => self.preview.show_message(
                    "Selected display",
                    &format!(
                        "{display_label}\n\nLive desktop frames need portal capture in a later milestone. This is the selected monitor, not a fake picture."
                    ),
                ),
                _ => {}
            }
        } else if let Some(camera) = camera {
            let key = (camera.path.clone(), mirror);
            let live = self.live.borrow_mut();
            if live.cam_key.as_ref() != Some(&key) {
                drop(live);
                self.stop_preview();
                let stop = Arc::new(AtomicBool::new(false));
                let rx = preview_live::start_camera(camera, mirror, Arc::clone(&stop));
                let mut live = self.live.borrow_mut();
                live.preview_stop = Some(stop);
                live.preview_rx = Some(rx);
                live.cam_key = Some(key);
                self.preview
                    .show_message("Starting camera…", "Opening the selected device.");
            }
        } else {
            self.stop_preview();
            let message = inventory
                .errors
                .iter()
                .find(|e| e.to_ascii_lowercase().contains("camera"))
                .cloned()
                .unwrap_or_else(|| "Select a camera or plug one in.".into());
            self.preview.show_message("No camera", &message);
        }

        self.ensure_meter(&mic, &mic_label, true);
        self.ensure_meter(&desk, &desk_label, false);
    }

    fn ensure_meter(&self, target: &Option<String>, label: &str, mic: bool) {
        let mut live = self.live.borrow_mut();
        let current = if mic {
            live.mic_key.clone()
        } else {
            live.desk_key.clone()
        };
        if current.as_ref() == target.as_ref() {
            return;
        }
        if mic {
            if let Some(stop) = live.mic_stop.take() {
                stop.store(true, Ordering::Relaxed);
            }
            live.mic_rx = None;
            live.mic_key = None;
        } else if let Some(stop) = live.desk_stop.take() {
            stop.store(true, Ordering::Relaxed);
            live.desk_rx = None;
            live.desk_key = None;
        }
        let Some(id) = target.clone() else {
            if mic {
                self.meters.mic.set_idle("No microphone selected.");
            } else {
                self.meters
                    .desktop
                    .set_idle("No desktop audio monitor selected.");
            }
            return;
        };
        let stop = Arc::new(AtomicBool::new(false));
        let rx = meter_live::start_tap(id.clone(), Arc::clone(&stop));
        if mic {
            live.mic_stop = Some(stop);
            live.mic_rx = Some(rx);
            live.mic_key = Some(id);
            self.meters.mic.set_idle(&format!("Listening to {label}…"));
        } else {
            live.desk_stop = Some(stop);
            live.desk_rx = Some(rx);
            live.desk_key = Some(id);
            self.meters
                .desktop
                .set_idle(&format!("Listening to {label}…"));
        }
    }

    fn stop_preview(&self) {
        let mut live = self.live.borrow_mut();
        if let Some(stop) = live.preview_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        live.preview_rx = None;
        live.cam_key = None;
    }

    fn drain_preview(&self) {
        let mut latest = None;
        {
            let live = self.live.borrow();
            let Some(rx) = live.preview_rx.as_ref() else {
                return;
            };
            while let Ok(event) = rx.try_recv() {
                latest = Some(event);
            }
        }
        match latest {
            Some(PreviewEvent::Frame { width, height, rgb }) => {
                self.preview.show_frame(width, height, &rgb);
            }
            Some(PreviewEvent::Error(message)) => self.preview.show_message("Camera error", &message),
            None => {}
        }
    }

    fn drain_meters(&self, state: &StudioState) {
        let settings = state.settings.borrow();
        let mic_label = settings
            .audio
            .mic_label
            .clone()
            .unwrap_or_else(|| "Microphone".into());
        let desk_label = settings
            .audio
            .desktop_label
            .clone()
            .unwrap_or_else(|| "Desktop".into());
        drop(settings);

        let mut mic_event = None;
        let mut desk_event = None;
        {
            let live = self.live.borrow();
            if let Some(rx) = live.mic_rx.as_ref() {
                while let Ok(event) = rx.try_recv() {
                    mic_event = Some(event);
                }
            }
            if let Some(rx) = live.desk_rx.as_ref() {
                while let Ok(event) = rx.try_recv() {
                    desk_event = Some(event);
                }
            }
        }
        match mic_event {
            Some(MeterEvent::Levels(levels)) => self.meters.mic.set_levels(levels, &mic_label),
            Some(MeterEvent::Error(err)) => self.meters.mic.set_error(&err),
            None => {}
        }
        match desk_event {
            Some(MeterEvent::Levels(levels)) => self.meters.desktop.set_levels(levels, &desk_label),
            Some(MeterEvent::Error(err)) => self.meters.desktop.set_error(&err),
            None => {}
        }
    }
}

fn persist_defaults(
    state: &StudioState,
    inventory: &DeviceInventory,
    displays: &[DisplaySource],
) {
    let mut changed = false;
    {
        let mut settings = state.settings.borrow_mut();
        if settings.camera.device.is_none() {
            if let Some(camera) = inventory.cameras.first() {
                settings.camera.device = Some(camera.path.clone());
                settings.camera.label = Some(camera.name.clone());
                if let Some(fmt) = camera.preferred_format() {
                    settings.camera.width = fmt.width;
                    settings.camera.height = fmt.height;
                    settings.camera.pixel_format = Some(fmt.pixel_format.clone());
                }
                changed = true;
            }
        }
        if settings.audio.mic_device.is_none() {
            if let Some(mic) = inventory.microphones.first() {
                settings.audio.mic_device = Some(mic.id.clone());
                settings.audio.mic_label = Some(mic.name.clone());
                changed = true;
            }
        }
        if settings.audio.desktop_device.is_none() {
            if let Some(desk) = inventory.desktop_audio.first() {
                settings.audio.desktop_device = Some(desk.id.clone());
                settings.audio.desktop_label = Some(desk.name.clone());
                changed = true;
            }
        }
        if settings.video.display_id.is_none() {
            if let Some(display) = displays.iter().find(|d| d.primary).or_else(|| displays.first())
            {
                settings.video.display_id = Some(display.id.clone());
                settings.video.display_label = Some(display.geometry_label());
                changed = true;
            }
        }
    }
    if changed {
        let _ = state.persist();
    }
}

fn gdk_displays() -> Vec<DisplaySource> {
    let Some(display) = gtk::gdk::Display::default() else {
        return Vec::new();
    };
    let monitors = display.monitors();
    let mut out = Vec::new();
    for index in 0..monitors.n_items() {
        let Some(item) = monitors.item(index) else {
            continue;
        };
        let Ok(monitor) = item.downcast::<gtk::gdk::Monitor>() else {
            continue;
        };
        let geo = monitor.geometry();
        let connector = monitor
            .connector()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Display {}", index + 1));
        let model = monitor.model().map(|s| s.to_string()).unwrap_or_default();
        let label = if model.is_empty() {
            connector.clone()
        } else {
            format!("{connector} · {model}")
        };
        out.push(DisplaySource {
            id: format!("{index}:{connector}"),
            label,
            width: geo.width() as u32,
            height: geo.height() as u32,
            x: geo.x(),
            y: geo.y(),
            scale: monitor.scale_factor() as f64,
            primary: index == 0,
        });
    }
    out
}
