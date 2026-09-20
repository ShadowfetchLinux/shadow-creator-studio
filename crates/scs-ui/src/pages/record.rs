use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use adw::prelude::*;
use gtk::gdk::prelude::MonitorExt;
use gtk::gio::prelude::ListModelExt;
use scs_capture::{DeviceInventory, DisplaySource, PipCorner, PipSpec};
use scs_core::RecordingMode;
use scs_core::settings::EnginePreference;
use scs_ffmpeg::{DesktopVideoInput, GstPip};
use scs_system::SystemSnapshot;

use crate::live::meters::{self as meter_live, MeterEvent};
use crate::live::plan;
use crate::live::preview::{self as preview_live, PreviewEvent};
use crate::live::record::{self as rec_live, RecordEvent};
use crate::state::StudioState;
use crate::widgets::{dashboard, meters, mode_tiles, preview, sources, status_strip, tracks};
use scs_core::{stop_requires_confirmation_pref, MarkerFile};

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
    window: adw::ApplicationWindow,
    rec_btn: gtk::Button,
    rec_note: gtk::Label,
    live_btn: gtk::Button,
    live_note: gtk::Label,
    rec_stop: RefCell<Option<Arc<AtomicBool>>>,
    rec_rx: RefCell<Option<Receiver<RecordEvent>>>,
    tracks: tracks::TrackMixer,
    last_mic_peak: Cell<f32>,
    share_btn: gtk::Button,
    share_note: gtk::Label,
    portal_rx: RefCell<Option<Receiver<Result<crate::live::portal::SharedDesktop, String>>>>,
    pending_record: Cell<bool>,
    pending_live: Cell<bool>,
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
    pub fn new(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> Self {
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
        let tracks = tracks::TrackMixer::build(state);
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
        rec.set_halign(gtk::Align::Center);

        let share_btn = gtk::Button::with_label("Share screen");
        share_btn.set_halign(gtk::Align::Center);
        share_btn.set_tooltip_text(Some(
            "Opens the desktop portal so Screen and Presentation can capture a monitor or window.",
        ));
        let share_note = gtk::Label::new(Some(
            "Screen and Presentation use xdg-desktop-portal + GStreamer (PipeWire). Region capture is not offered.",
        ));
        share_note.add_css_class("scs-unavailable");
        share_note.set_halign(gtk::Align::Center);
        share_note.set_wrap(true);

        let rec_note = gtk::Label::new(Some(
            "Camera, Voice, and Creator write an MKV. Screen and Presentation use the desktop portal.",
        ));
        rec_note.add_css_class("scs-unavailable");
        rec_note.set_halign(gtk::Align::Center);
        rec_note.set_wrap(true);

        let live_card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        live_card.add_css_class("scs-card");
        let live_title = gtk::Label::new(Some("GO LIVE"));
        live_title.add_css_class("heading");
        live_title.set_halign(gtk::Align::Start);
        let live_btn = gtk::Button::with_label("GO LIVE");
        live_btn.add_css_class("scs-live-button");
        live_btn.set_sensitive(false);
        live_btn.set_halign(gtk::Align::Start);
        live_btn.set_tooltip_text(Some("Stores a key and passes the RTMP probe first."));
        let live_note = gtk::Label::new(Some(
            "GO LIVE needs a keyring-backed stream key and an FFmpeg flv muxer. The key is never logged.",
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
        column.append(&tracks.root);
        column.append(&status.root);
        column.append(&share_btn);
        column.append(&share_note);
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
            window: window.clone(),
            rec_btn: rec,
            rec_note,
            live_btn,
            live_note,
            rec_stop: RefCell::new(None),
            rec_rx: RefCell::new(None),
            tracks,
            last_mic_peak: Cell::new(-90.0),
            share_btn,
            share_note,
            portal_rx: RefCell::new(None),
            pending_record: Cell::new(false),
            pending_live: Cell::new(false),
        }
    }

    pub fn connect(&self, page: &Rc<Self>, state: &Rc<StudioState>) {
        let page_c = Rc::clone(page);
        let state_c = Rc::clone(state);
        self.rec_btn.connect_clicked(move |_| {
            page_c.on_record_clicked(&state_c);
        });
        let page_l = Rc::clone(page);
        let state_l = Rc::clone(state);
        self.live_btn.connect_clicked(move |_| {
            if let Err(err) = page_l.start_live(&state_l) {
                page_l.alert("Cannot go live", &err);
            }
        });
        let page_k = Rc::clone(page);
        let state_k = Rc::clone(state);
        self.tracks.calibrate.connect_clicked(move |_| {
            page_k.calibrate_mic(&state_k);
        });
        let page_s = Rc::clone(page);
        let state_s = Rc::clone(state);
        self.share_btn.connect_clicked(move |_| {
            page_s.begin_share(&state_s, false, false);
        });
        self.refresh_record_chrome(state);
        self.refresh_live_chrome(state);
    }

    fn start_live(&self, state: &Rc<StudioState>) -> Result<(), String> {
        if state.recording.borrow().active {
            return Err("Stop the current take first. Live uses a new session.".into());
        }
        if state.settings.borrow().advanced.engine == EnginePreference::Obs {
            return self.start_obs(state, true);
        }
        let key = scs_core::lookup_stream_key()?.ok_or_else(|| {
            "Store a YouTube stream key in Settings (system keyring) first.".to_string()
        })?;
        let streaming = state.settings.borrow().streaming.clone();
        let url = scs_ffmpeg::build_rtmp_url(&streaming.server_url, &key, streaming.rtmps)?;
        let mode = state.settings.borrow().last_recording_mode;
        if mode.needs_desktop_share() {
            if state.desktop_share.borrow().is_none() {
                self.begin_share(state, false, true);
                return Ok(());
            }
            return self.start_desktop_take(state, Some(url));
        }
        let disk = state.monitor.borrow_mut().snapshot().disk;
        let request = crate::live::plan::attach_stream(
            plan::build_request(state, &self.inventory.borrow(), disk)?,
            Some(url),
            streaming.record_while_live,
        );
        let remux = state.settings.borrow().recording.remux_to_mp4 && streaming.record_while_live;
        self.stop_preview();
        let stop = Arc::new(AtomicBool::new(false));
        let rx = rec_live::start_session(request, remux, Arc::clone(&stop));
        self.mark_session_started(state, stop, rx);
        state.recording.borrow_mut().last_message = Some("Status: Going live…".into());
        Ok(())
    }

    fn refresh_live_chrome(&self, state: &StudioState) {
        let key_present = state.settings.borrow().streaming.stream_key_ref.is_some();
        let muxers = state.muxers.borrow();
        let probe = scs_ffmpeg::evaluate_probe(key_present, Ok("rtmp://placeholder/live"), &muxers);
        if state.recording.borrow().active {
            self.live_btn.set_sensitive(false);
            self.live_btn
                .set_tooltip_text(Some("A session is already running."));
            let dropped = state
                .recording
                .borrow()
                .dropped
                .map(|n| format!("Stream health · dropped frames: {n}"))
                .unwrap_or_else(|| "Stream health: waiting for FFmpeg progress.".into());
            self.live_note.set_text(&dropped);
            return;
        }
        self.live_btn.set_sensitive(probe.ok && key_present);
        self.live_btn.set_tooltip_text(Some(probe.reason.as_str()));
        self.live_note.set_text(&probe.reason);
    }

    fn calibrate_mic(&self, state: &Rc<StudioState>) {
        let peak = self.last_mic_peak.get();
        let advice = scs_audio::recommend_from_peak(peak);
        {
            let mut settings = state.settings.borrow_mut();
            settings.audio.mic_volume = advice.recommended_volume;
            settings.audio.processing_preset = advice.suggested_preset;
        }
        let _ = state.persist();
        self.tracks.show_advice(&format!(
            "Peak {peak:+.1} dB → volume {:.2} · {} — {}",
            advice.recommended_volume,
            advice.suggested_preset.label(),
            advice.note
        ));
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
        self.watch_disk(state, snap);
        self.drain_portal(state);
        self.drain_record(state);
        self.sync_sessions(state);
        self.pump(state, snap);
        self.refresh_record_chrome(state);
        self.refresh_live_chrome(state);
    }

    fn begin_share(&self, state: &Rc<StudioState>, then_record: bool, then_live: bool) {
        if self.portal_rx.borrow().is_some() {
            self.share_note
                .set_text("A share dialog is already open. Choose a screen or cancel it.");
            return;
        }
        self.pending_record.set(then_record);
        self.pending_live.set(then_live);
        let want_window = state.settings.borrow().video.window_capture;
        *self.portal_rx.borrow_mut() = Some(crate::live::portal::request_share(want_window));
        self.share_note
            .set_text("Choose a monitor or window in the desktop portal dialog.");
        *state.desktop_note.borrow_mut() = Some("Waiting for the portal…".into());
    }

    fn drain_portal(&self, state: &Rc<StudioState>) {
        let result = self
            .portal_rx
            .borrow()
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        let Some(result) = result else { return };
        *self.portal_rx.borrow_mut() = None;
        match result {
            Ok(share) => {
                let label = format!(
                    "Shared {}×{} (node {})",
                    share.stream.width, share.stream.height, share.stream.node_id
                );
                self.share_note.set_text(&label);
                *state.desktop_note.borrow_mut() = Some(label);
                *state.desktop_share.borrow_mut() = Some(share);
                self.stop_preview();
                if self.pending_live.replace(false) {
                    if let Err(err) = self.start_live(state) {
                        self.alert("Cannot go live", &err);
                    }
                } else if self.pending_record.replace(false) {
                    if let Err(err) = self.start_take(state) {
                        self.alert("Cannot start recording", &err);
                    }
                }
            }
            Err(err) => {
                self.pending_record.set(false);
                self.pending_live.set(false);
                self.share_note.set_text(&err);
                *state.desktop_note.borrow_mut() = Some(err.clone());
                if err.to_ascii_lowercase().contains("cancel") {
                    return;
                }
                self.alert("Screen share failed", &err);
            }
        }
    }

    pub fn pump(&self, state: &StudioState, snap: &SystemSnapshot) {
        self.status.refresh(state, snap);
        self.dashboard.refresh(state, snap);
        self.drain_preview();
        self.drain_meters(state);
    }

    pub fn sync_sessions(&self, state: &Rc<StudioState>) {
        if state.recording.borrow().active {
            self.stop_preview();
            if let Some(path) = state.recording.borrow().path.as_ref() {
                self.preview.show_message(
                    "Recording",
                    &format!(
                        "Writing {}\nCamera preview is paused so the take can own the device.",
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string())
                    ),
                );
            }
            return;
        }
        let mode = state.settings.borrow().last_recording_mode;
        let inventory = self.inventory.borrow();
        let settings = state.settings.borrow();
        let want_cam = state.camera_preview.get()
            && matches!(
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

        if !state.camera_preview.get() {
            self.stop_preview();
            self.preview.show_message(
                "Camera preview off",
                "F6 toggles the preview. The take still uses the selected camera when you record.",
            );
            self.ensure_meter(&mic, &mic_label, true);
            self.ensure_meter(&desk, &desk_label, false);
            return;
        }
        if matches!(mode, RecordingMode::Screen | RecordingMode::Presentation) {
            if let Some(share) = state.desktop_share.borrow().as_ref() {
                let key = (format!("desk:{}", share.stream.node_id), false);
                let live = self.live.borrow_mut();
                if live.cam_key.as_ref() != Some(&key) {
                    drop(live);
                    self.stop_preview();
                    let input = DesktopVideoInput {
                        node_id: share.stream.node_id,
                        fd: share.fd,
                        width: share.stream.width,
                        height: share.stream.height,
                    };
                    let stop = Arc::new(AtomicBool::new(false));
                    let rx = preview_live::start_desktop(input, Arc::clone(&stop));
                    let mut live = self.live.borrow_mut();
                    live.preview_stop = Some(stop);
                    live.preview_rx = Some(rx);
                    live.cam_key = Some(key);
                    self.preview
                        .show_message("Starting desktop…", "Opening the shared PipeWire stream.");
                }
            } else {
                self.stop_preview();
                self.preview.show_message(
                    "Share a screen",
                    &format!(
                        "{display_label}\n\nClick Share screen (or Start Recording) to open the desktop portal. Live frames appear after you grant a monitor or window."
                    ),
                );
            }
            self.ensure_meter(&mic, &mic_label, true);
            self.ensure_meter(&desk, &desk_label, false);
            return;
        }
        if !want_cam {
            self.stop_preview();
            match mode {
                RecordingMode::Voice => self.preview.show_message(
                    "Voice mode",
                    "Camera preview is off. Meters stay live so you can set level.",
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

    pub fn on_hotkey_record(&self, state: &Rc<StudioState>) {
        self.on_record_clicked(state);
    }

    fn on_record_clicked(&self, state: &Rc<StudioState>) {
        if state.recording.borrow().active {
            if stop_requires_confirmation_pref(state.settings.borrow().hotkeys.confirm_stop) {
                let dialog = adw::AlertDialog::new(
                    Some("Stop recording?"),
                    Some("The MKV is kept. Stopping does not delete the take."),
                );
                dialog.add_response("cancel", "Keep recording");
                dialog.add_response("stop", "Stop");
                dialog.set_response_appearance("stop", adw::ResponseAppearance::Destructive);
                let stop = self.rec_stop.borrow().as_ref().map(Arc::clone);
                dialog.connect_response(None, move |_, response| {
                    if response == "stop" {
                        if let Some(stop) = &stop {
                            stop.store(true, Ordering::Relaxed);
                        }
                    }
                });
                dialog.present(Some(&self.window));
            } else if let Some(stop) = self.rec_stop.borrow().as_ref() {
                stop.store(true, Ordering::Relaxed);
            }
            return;
        }
        if let Err(err) = self.start_take(state) {
            self.alert("Cannot start recording", &err);
        }
    }

    fn start_take(&self, state: &Rc<StudioState>) -> Result<(), String> {
        let mode = state.settings.borrow().last_recording_mode;
        let engine = state.settings.borrow().advanced.engine;
        if engine == EnginePreference::Obs {
            return self.start_obs(state, false);
        }
        if mode.needs_desktop_share() {
            if state.desktop_share.borrow().is_none() {
                self.begin_share(state, true, false);
                return Ok(());
            }
            return self.start_desktop_take(state, None);
        }
        let disk = state.monitor.borrow_mut().snapshot().disk;
        let request = plan::build_request(state, &self.inventory.borrow(), disk)?;
        let remux = state.settings.borrow().recording.remux_to_mp4;
        self.stop_preview();
        let stop = Arc::new(AtomicBool::new(false));
        let rx = rec_live::start_session(request, remux, Arc::clone(&stop));
        self.mark_session_started(state, stop, rx);
        Ok(())
    }

    fn start_desktop_take(&self, state: &Rc<StudioState>, rtmp: Option<String>) -> Result<(), String> {
        let share = state
            .desktop_share
            .borrow()
            .clone()
            .ok_or_else(|| "Share a screen first.".to_string())?;
        if !scs_ffmpeg::gst_available() {
            return Err("Install gstreamer1.0-tools (gst-launch-1.0) to record the desktop.".into());
        }
        let disk = state.monitor.borrow_mut().snapshot().disk;
        let request = plan::build_request(state, &self.inventory.borrow(), disk)?;
        let settings = state.settings.borrow();
        let pip = if settings.last_recording_mode == RecordingMode::Presentation {
            let camera = request
                .camera
                .as_ref()
                .ok_or_else(|| "Presentation needs a camera for the picture-in-picture.".to_string())?;
            let spec = PipSpec {
                corner: PipCorner::from_key(&settings.video.pip_corner),
                width: 480,
                height: 270,
            };
            let (x, y) = spec.corner.offset(
                share.stream.width,
                share.stream.height,
                spec.width,
                spec.height,
                24,
            );
            Some(GstPip {
                camera_path: camera.path.clone(),
                x,
                y,
                width: spec.width,
                height: spec.height,
            })
        } else {
            None
        };
        let remux = settings.recording.remux_to_mp4
            && (rtmp.is_none() || settings.streaming.record_while_live);
        let prefer_nvenc = state.caps.h264_nvenc.is_available();
        drop(settings);
        self.stop_preview();
        let stop = Arc::new(AtomicBool::new(false));
        let rx = rec_live::start_desktop_session(
            DesktopVideoInput {
                node_id: share.stream.node_id,
                fd: share.fd,
                width: share.stream.width,
                height: share.stream.height,
            },
            request.mic.clone(),
            pip,
            request.output.clone(),
            prefer_nvenc,
            rtmp,
            remux,
            Arc::clone(&stop),
        );
        self.mark_session_started(state, stop, rx);
        Ok(())
    }

    fn start_obs(&self, state: &Rc<StudioState>, stream: bool) -> Result<(), String> {
        self.stop_preview();
        let stop = Arc::new(AtomicBool::new(false));
        let rx = rec_live::start_obs_session(stream, Arc::clone(&stop));
        self.mark_session_started(state, stop, rx);
        Ok(())
    }

    fn mark_session_started(
        &self,
        state: &StudioState,
        stop: Arc<AtomicBool>,
        rx: Receiver<RecordEvent>,
    ) {
        *self.rec_stop.borrow_mut() = Some(stop);
        *self.rec_rx.borrow_mut() = Some(rx);
        let mut rec = state.recording.borrow_mut();
        rec.active = true;
        rec.started = Some(std::time::Instant::now());
        rec.warning = None;
        rec.dropped = None;
        rec.last_message = Some("Status: Starting…".into());
        rec.last_marker = None;
        *state.markers.borrow_mut() = MarkerFile::new("pending");
    }

    fn drain_record(&self, state: &Rc<StudioState>) {
        let mut events = Vec::new();
        if let Some(rx) = self.rec_rx.borrow().as_ref() {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                RecordEvent::Started { path, encoder } => {
                    let mut rec = state.recording.borrow_mut();
                    rec.active = true;
                    rec.path = Some(path);
                    rec.encoder = encoder.clone();
                    rec.started = rec.started.or(Some(std::time::Instant::now()));
                    *state.encoder_status.borrow_mut() = format!("{encoder} · recording");
                }
                RecordEvent::Progress {
                    frame,
                    out_time_ms,
                    drop_frames,
                } => {
                    let mut rec = state.recording.borrow_mut();
                    if rec.started.is_none() && out_time_ms > 0 {
                        rec.started = Some(
                            std::time::Instant::now()
                                - std::time::Duration::from_millis(out_time_ms),
                        );
                    }
                    rec.dropped = drop_frames.or(rec.dropped);
                    if frame > 0 && rec.warning.is_none() {
                        rec.last_message = Some(format!("Status: Recording · {frame} frames"));
                    }
                }
                RecordEvent::Warning(message) => {
                    state.recording.borrow_mut().warning = Some(message);
                }
                RecordEvent::Finished { message, remuxed } => {
                    let extra = remuxed
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .map(|name| format!(" · {name}"))
                        .unwrap_or_default();
                    self.finish_take(state, &format!("Status: Idle · {message}{extra}"));
                }
                RecordEvent::Failed(message) => {
                    self.finish_take(state, &format!("Status: Idle · {message}"));
                    self.alert("Recording failed", &message);
                }
            }
        }
    }

    fn finish_take(&self, state: &StudioState, status: &str) {
        *self.rec_stop.borrow_mut() = None;
        *self.rec_rx.borrow_mut() = None;
        let encoder = state
            .recording
            .borrow()
            .encoder
            .clone();
        let mut rec = state.recording.borrow_mut();
        rec.active = false;
        rec.started = None;
        rec.last_message = Some(status.into());
        rec.warning = None;
        if !encoder.is_empty() {
            *state.encoder_status.borrow_mut() = format!("{encoder} ready · idle");
        }
    }

    fn watch_disk(&self, state: &StudioState, snap: &SystemSnapshot) {
        if !state.recording.borrow().active {
            return;
        }
        let Some(disk) = snap.disk else {
            return;
        };
        if disk.emergency_stop() {
            state.recording.borrow_mut().warning =
                Some("Disk almost full — stopping to protect the take".into());
            if let Some(stop) = self.rec_stop.borrow().as_ref() {
                stop.store(true, Ordering::Relaxed);
            }
        } else if disk.warn_low() {
            state.recording.borrow_mut().warning = Some("Low disk space".into());
        }
    }

    fn refresh_record_chrome(&self, state: &StudioState) {
        let mode = state.settings.borrow().last_recording_mode;
        let recording = state.recording.borrow().active;
        if recording {
            self.rec_btn.set_label("STOP RECORDING");
            self.rec_btn.set_sensitive(true);
            self.rec_btn
                .set_tooltip_text(Some("Asks for confirmation before stopping"));
            let marker = state
                .recording
                .borrow()
                .last_marker
                .clone()
                .unwrap_or_else(|| "F8 drops a chapter marker.".into());
            self.rec_note.set_text(&format!(
                "Recording. Stop asks for confirmation unless you disabled it. {marker}"
            ));
            return;
        }
        self.rec_btn.set_label("START RECORDING");
        if let Some(reason) = mode.unavailable_reason() {
            self.rec_btn.set_sensitive(false);
            self.rec_btn.set_tooltip_text(Some(reason));
            self.rec_note.set_text(reason);
        } else {
            self.rec_btn.set_sensitive(true);
            self.rec_btn.set_tooltip_text(Some(
                "Writes a crash-safe MKV. Screen modes use the desktop portal + GStreamer.",
            ));
            self.rec_note.set_text(
                "Camera, Voice, and Creator use FFmpeg. Screen and Presentation use the portal + GStreamer. OBS is optional in Settings.",
            );
        }
    }

    pub fn drop_marker(&self, state: &StudioState) {
        if !state.recording.borrow().active {
            state.recording.borrow_mut().last_marker =
                Some("Markers are stored on the current take. Start recording first.".into());
            return;
        }
        let elapsed = state
            .recording
            .borrow()
            .started
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let path = state.recording.borrow().path.clone();
        let marker = {
            let mut file = state.markers.borrow_mut();
            if file.recording_id == "idle" {
                *file = MarkerFile::new(
                    path.as_ref()
                        .and_then(|p| p.file_stem())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "take".into()),
                );
            }
            let n = file.markers.len() + 1;
            file.add(elapsed, format!("Marker {n}")).clone()
        };
        if let Some(path) = path {
            let side = MarkerFile::path_for(&path);
            let _ = state.markers.borrow().save(&side);
            state.recording.borrow_mut().last_marker = Some(format!(
                "Marker at {:.1}s · {}",
                elapsed as f64 / 1000.0,
                side.display()
            ));
        }
        let _ = marker;
    }

    fn alert(&self, title: &str, body: &str) {
        let dialog = adw::AlertDialog::new(Some(title), Some(body));
        dialog.add_response("ok", "OK");
        dialog.present(Some(&self.window));
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
            Some(MeterEvent::Levels(levels)) => {
                self.last_mic_peak.set(levels.peak_db);
                self.meters.mic.set_levels(levels, &mic_label);
            }
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
