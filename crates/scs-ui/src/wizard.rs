use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use scs_capture::DeviceInventory;
use scs_core::quality::QualityPreset;
use scs_diagnostics::collect_report;
use scs_system::SystemSnapshot;

use crate::live;
use crate::state::StudioState;

const STEP_TITLES: [&str; 8] = [
    "Welcome",
    "Recording folder",
    "Quality",
    "Camera",
    "Microphone",
    "Desktop audio",
    "Hardware",
    "Review",
];

pub fn present(parent: &adw::ApplicationWindow, state: &Rc<StudioState>) {
    let window = adw::Window::new();
    window.set_transient_for(Some(parent));
    window.set_modal(true);
    window.set_default_width(640);
    window.set_default_height(520);
    window.set_title(Some("First-run setup"));

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar.add_top_bar(&header);

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

    let folder_subtitle = gtk::Label::new(Some(
        state
            .settings
            .borrow()
            .recording
            .folder
            .display()
            .to_string()
            .as_str(),
    ));
    folder_subtitle.set_wrap(true);

    let camera_page = DeviceStep::build(
        "Camera",
        "Names and formats come from a background scan. Preview lives on the Record page — this step does not run a fake pass.",
    );
    let mic_page = DeviceStep::build(
        "Microphone",
        "PipeWire sources are listed when pw-dump succeeds. A real meter runs on the Record page.",
    );
    let desk_page = DeviceStep::build(
        "Desktop audio",
        "Monitor sources appear when PipeWire reports a sink or loopback. System mic routing is never rewritten.",
    );

    stack.add_named(&welcome_page(), Some("0"));
    stack.add_named(&folder_page(state, &window, &folder_subtitle), Some("1"));
    stack.add_named(&quality_page(state), Some("2"));
    stack.add_named(&camera_page.root, Some("3"));
    stack.add_named(&mic_page.root, Some("4"));
    stack.add_named(&desk_page.root, Some("5"));
    stack.add_named(&hardware_page(state), Some("6"));
    stack.add_named(&review_page(state), Some("7"));
    stack.add_named(&ready_page(), Some("ready"));

    let step = Rc::new(Cell::new(0usize));
    let status = gtk::Label::new(Some("Step 1 of 8 — Welcome"));
    status.add_css_class("dim-label");

    let back = gtk::Button::with_label("Back");
    let next = gtk::Button::with_label("Next");
    next.add_css_class("suggested-action");

    let nav = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    nav.set_halign(gtk::Align::End);
    nav.set_margin_top(12);
    nav.set_margin_bottom(16);
    nav.set_margin_end(20);
    nav.append(&back);
    nav.append(&next);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    stack.set_vexpand(true);
    body.append(&stack);
    status.set_margin_start(20);
    body.append(&status);
    body.append(&nav);
    toolbar.set_content(Some(&body));
    window.set_content(Some(&toolbar));

    let stack_b = stack.clone();
    let status_b = status.clone();
    let next_b = next.clone();
    let step_b = Rc::clone(&step);
    back.connect_clicked(move |_| {
        let current = step_b.get();
        if current == 0 || current == usize::MAX {
            return;
        }
        let next_idx = current.saturating_sub(1);
        step_b.set(next_idx);
        stack_b.set_visible_child_name(&next_idx.to_string());
        status_b.set_text(&format!(
            "Step {} of 8 — {}",
            next_idx + 1,
            STEP_TITLES[next_idx]
        ));
        next_b.set_label("Next");
        next_b.set_sensitive(true);
    });

    let stack_n = stack.clone();
    let status_n = status.clone();
    let state_n = Rc::clone(state);
    let window_n = window.clone();
    next.connect_clicked(move |btn| {
        let current = step.get();
        if current == usize::MAX {
            window_n.close();
            return;
        }
        if current + 1 < STEP_TITLES.len() {
            let next_idx = current + 1;
            step.set(next_idx);
            stack_n.set_visible_child_name(&next_idx.to_string());
            status_n.set_text(&format!(
                "Step {} of 8 — {}",
                next_idx + 1,
                STEP_TITLES[next_idx]
            ));
            if next_idx + 1 == STEP_TITLES.len() {
                btn.set_label("Finish setup");
            }
            return;
        }
        let _ = state_n.complete_wizard();
        step.set(usize::MAX);
        stack_n.set_visible_child_name("ready");
        status_n.set_text("You're ready to preview.");
        btn.set_label("Close");
    });

    let discover: Rc<RefCell<Option<Receiver<DeviceInventory>>>> =
        Rc::new(RefCell::new(Some(live::spawn_discover())));
    glib::timeout_add_local(Duration::from_millis(200), move || {
        let borrowed = discover.borrow();
        let Some(rx) = borrowed.as_ref() else {
            return glib::ControlFlow::Break;
        };
        match rx.try_recv() {
            Ok(inventory) => {
                drop(borrowed);
                camera_page.apply_cameras(&inventory);
                mic_page.apply_mics(&inventory);
                desk_page.apply_desktop(&inventory);
                *discover.borrow_mut() = None;
                glib::ControlFlow::Break
            }
            Err(_) => glib::ControlFlow::Continue,
        }
    });

    window.present();
}

struct DeviceStep {
    root: gtk::Box,
    list: gtk::Box,
}

impl DeviceStep {
    fn build(title: &str, body: &str) -> Rc<Self> {
        let root = page_frame(title, body);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let scanning = gtk::Label::new(Some("Scanning in the background…"));
        scanning.add_css_class("dim-label");
        scanning.set_halign(gtk::Align::Start);
        list.append(&scanning);
        let note = gtk::Label::new(Some(
            "Use the Record page for live preview and meters. This wizard does not fake a test.",
        ));
        note.add_css_class("scs-unavailable");
        note.set_halign(gtk::Align::Start);
        note.set_wrap(true);
        root.append(&list);
        root.append(&note);
        Rc::new(Self { root, list })
    }

    fn replace_list(&self, lines: &[String]) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        if lines.is_empty() {
            let empty = gtk::Label::new(Some("Unavailable — nothing was found."));
            empty.add_css_class("scs-unavailable");
            empty.set_halign(gtk::Align::Start);
            empty.set_wrap(true);
            self.list.append(&empty);
            return;
        }
        for line in lines {
            let row = gtk::Label::new(Some(line));
            row.set_halign(gtk::Align::Start);
            row.set_wrap(true);
            self.list.append(&row);
        }
    }

    fn apply_cameras(&self, inventory: &DeviceInventory) {
        let mut lines: Vec<String> = inventory
            .cameras
            .iter()
            .map(|camera| camera.label())
            .collect();
        if lines.is_empty() {
            lines.extend(
                inventory
                    .errors
                    .iter()
                    .filter(|e| e.to_ascii_lowercase().contains("camera"))
                    .cloned(),
            );
        }
        self.replace_list(&lines);
    }

    fn apply_mics(&self, inventory: &DeviceInventory) {
        let mut lines: Vec<String> = inventory
            .microphones
            .iter()
            .map(|mic| mic.name.clone())
            .collect();
        if lines.is_empty() {
            lines.extend(
                inventory
                    .errors
                    .iter()
                    .filter(|e| e.to_ascii_lowercase().contains("microphone"))
                    .cloned(),
            );
        }
        self.replace_list(&lines);
    }

    fn apply_desktop(&self, inventory: &DeviceInventory) {
        let mut lines: Vec<String> = inventory
            .desktop_audio
            .iter()
            .map(|desk| desk.name.clone())
            .collect();
        if lines.is_empty() {
            lines.extend(
                inventory
                    .errors
                    .iter()
                    .filter(|e| e.to_ascii_lowercase().contains("desktop"))
                    .cloned(),
            );
        }
        self.replace_list(&lines);
    }
}

fn welcome_page() -> gtk::Box {
    page_frame(
        "Welcome to Shadow Creator Studio",
        "A simple creator surface on top of OBS, FFmpeg, PipeWire, and NVIDIA NVENC.\n\nMilestone 2 lists cameras and PipeWire audio and previews them on the Record page. START RECORDING and GO LIVE stay disabled until later milestones.",
    )
}

fn folder_page(
    state: &Rc<StudioState>,
    window: &adw::Window,
    subtitle: &gtk::Label,
) -> gtk::Box {
    let root = page_frame(
        "Where should recordings go?",
        "This folder is saved now. Files will use timestamped names like 2026-09-20_YouTube_Record_001.mkv.",
    );
    subtitle.set_halign(gtk::Align::Start);
    subtitle.add_css_class("scs-gold");
    let browse = gtk::Button::with_label("Choose folder");
    let window = window.clone();
    let state = Rc::clone(state);
    let subtitle_btn = subtitle.clone();
    browse.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Recording folder")
            .build();
        let state = Rc::clone(&state);
        let subtitle = subtitle_btn.clone();
        dialog.select_folder(Some(&window), gtk::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    subtitle.set_text(&path.display().to_string());
                    let _ = state.set_folder(path);
                }
            }
        });
    });
    root.append(subtitle);
    root.append(&browse);
    root
}

fn quality_page(state: &Rc<StudioState>) -> gtk::Box {
    let root = page_frame(
        "Quality preset",
        "YouTube-friendly defaults. This updates resolution, FPS, and disk-time estimates.",
    );
    let current = state.settings.borrow().recording.quality;
    let mut group: Option<gtk::CheckButton> = None;
    for preset in QualityPreset::ALL {
        let btn = gtk::CheckButton::with_label(preset.label());
        if let Some(ref lead) = group {
            btn.set_group(Some(lead));
        } else {
            group = Some(btn.clone());
        }
        if preset == current {
            btn.set_active(true);
        }
        let state = Rc::clone(state);
        btn.connect_toggled(move |b| {
            if b.is_active() {
                let mut settings = state.settings.borrow_mut();
                settings.recording.quality = preset;
                settings.video.apply_quality(preset);
                drop(settings);
                let _ = state.persist();
            }
        });
        root.append(&btn);
    }
    root
}

fn hardware_page(state: &StudioState) -> gtk::Box {
    let root = page_frame(
        "Hardware check",
        "These rows are real probes. Anything not implemented stays Unavailable.",
    );
    let snap: SystemSnapshot = state.monitor.borrow_mut().snapshot();
    let report = collect_report(&state.settings.borrow(), Some(&snap));
    for (title, value) in [
        ("OS", report.os.display()),
        ("GPU", report.gpu.display()),
        ("FFmpeg", report.ffmpeg.display()),
        ("PipeWire", report.pipewire.display()),
        ("NVENC", cap(&report.encoder_capabilities.h264_nvenc)),
    ] {
        let row = gtk::Label::new(Some(&format!("{title}: {value}")));
        row.set_halign(gtk::Align::Start);
        row.set_wrap(true);
        root.append(&row);
    }
    root
}

fn review_page(state: &StudioState) -> gtk::Box {
    let settings = state.settings.borrow();
    let body = format!(
        "Folder: {}\nQuality: {}\nMode: {}\nCamera: {}\nMic: {}\nDesktop: {}\nContainer: MKV (crash-safe)\nEngine plan: OBS first, FFmpeg fallback\n\nRecording and GO LIVE stay unavailable until later milestones.",
        settings.recording.folder.display(),
        settings.recording.quality.label(),
        settings.last_recording_mode.label(),
        settings.camera.label.clone().unwrap_or_else(|| "not selected yet".into()),
        settings.audio.mic_label.clone().unwrap_or_else(|| "not selected yet".into()),
        settings.audio.desktop_label.clone().unwrap_or_else(|| "not selected yet".into()),
    );
    page_frame("Review", &body)
}

fn ready_page() -> gtk::Box {
    page_frame(
        "You're ready to preview.",
        "The studio can list devices and show a live camera plus meters. Press Close, then use Record.\n\nSTART RECORDING remains disabled until Milestone 3 — that is intentional.",
    )
}

fn page_frame(title: &str, body: &str) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(20);
    root.set_margin_start(24);
    root.set_margin_end(24);
    let heading = gtk::Label::new(Some(title));
    heading.add_css_class("title-2");
    heading.set_halign(gtk::Align::Start);
    heading.set_wrap(true);
    let text = gtk::Label::new(Some(body));
    text.set_wrap(true);
    text.set_halign(gtk::Align::Start);
    text.set_xalign(0.0);
    root.append(&heading);
    root.append(&text);
    root
}

fn cap(c: &scs_encoder::Capability) -> String {
    match c {
        scs_encoder::Capability::Available { notes } => format!("Available — {notes}"),
        scs_encoder::Capability::Unavailable { reason } => format!("Unavailable — {reason}"),
        scs_encoder::Capability::Unknown => "Unavailable — not probed".into(),
    }
}
