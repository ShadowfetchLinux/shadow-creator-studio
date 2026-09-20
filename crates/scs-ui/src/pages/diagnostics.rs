use std::rc::Rc;

use adw::prelude::*;
use gtk::prelude::*;
use scs_core::Settings;
use scs_diagnostics::{collect_report, DiagnosticReport};
use scs_system::SystemSnapshot;

use crate::state::StudioState;

pub struct DiagnosticsPage {
    pub root: gtk::ScrolledWindow,
    list: gtk::ListBox,
}

impl DiagnosticsPage {
    pub fn new(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> Self {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
        column.set_margin_top(20);
        column.set_margin_bottom(24);
        column.set_margin_start(24);
        column.set_margin_end(24);

        let heading = gtk::Label::new(Some("Diagnostics"));
        heading.add_css_class("title-1");
        heading.set_halign(gtk::Align::Start);
        let note = gtk::Label::new(Some(
            "Probes run without sudo. Missing detectors show Unavailable — never a fake pass.",
        ));
        note.add_css_class("dim-label");
        note.set_wrap(true);
        note.set_halign(gtk::Align::Start);

        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);

        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let refresh = gtk::Button::with_label("Refresh");
        let copy = gtk::Button::with_label("Copy diagnostic report");
        copy.add_css_class("suggested-action");
        buttons.append(&refresh);
        buttons.append(&copy);

        let list_refresh = list.clone();
        let state_r = Rc::clone(state);
        refresh.connect_clicked(move |_| {
            fill(&list_refresh, &state_r);
        });

        let state_c = Rc::clone(state);
        let window = window.clone();
        copy.connect_clicked(move |_| {
            let report = build_report(&state_c);
            let text = report.render_text();
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&text);
            }
            let dialog = adw::AlertDialog::new(
                Some("Diagnostic report copied"),
                Some("The clipboard contains a redacted report. Stream keys and tokens are stripped."),
            );
            dialog.add_response("ok", "OK");
            dialog.present(Some(&window));
        });

        column.append(&heading);
        column.append(&note);
        column.append(&list);
        column.append(&buttons);

        fill(&list, state);

        let root = gtk::ScrolledWindow::new();
        root.set_child(Some(&column));
        Self { root, list }
    }

    pub fn refresh(&self, state: &Rc<StudioState>) {
        fill(&self.list, state);
    }
}

fn fill(list: &gtk::ListBox, state: &StudioState) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let report = build_report(state);
    let rows = [
        ("Operating system", report.os.display()),
        ("Rustc", report.rustc.display()),
        ("FFmpeg", report.ffmpeg.display()),
        ("PipeWire", report.pipewire.display()),
        ("PipeWire socket", report.pipewire_socket.display()),
        ("GPU", report.gpu.display()),
        ("NVML", report.nvml.display()),
        ("OBS", report.obs.display()),
        ("EasyEffects", report.easyeffects.display()),
        (
            "h264_nvenc",
            cap(&report.encoder_capabilities.h264_nvenc),
        ),
        (
            "hevc_nvenc",
            cap(&report.encoder_capabilities.hevc_nvenc),
        ),
        ("av1_nvenc", cap(&report.encoder_capabilities.av1_nvenc)),
    ];
    for (title, value) in rows {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(value)
            .build();
        list.append(&row);
    }
}

fn cap(c: &scs_encoder::Capability) -> String {
    match c {
        scs_encoder::Capability::Available { notes } => format!("Available — {notes}"),
        scs_encoder::Capability::Unavailable { reason } => format!("Unavailable — {reason}"),
        scs_encoder::Capability::Unknown => "Unavailable — not probed".into(),
    }
}

fn build_report(state: &StudioState) -> DiagnosticReport {
    let settings: Settings = state.settings.borrow().clone();
    let snap: SystemSnapshot = state.monitor.borrow_mut().snapshot();
    collect_report(&settings, Some(&snap))
}
