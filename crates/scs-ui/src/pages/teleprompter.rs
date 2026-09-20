use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use glib;
use gtk::prelude::*;
use scs_teleprompter::{clamp_font, clamp_speed, TeleprompterScript};

use crate::state::StudioState;

#[allow(dead_code)]
pub struct TeleprompterPage {
    pub root: gtk::Box,
    pub view: gtk::TextView,
    scroll: gtk::ScrolledWindow,
    paused: gtk::CheckButton,
    mirrored: gtk::CheckButton,
    status: gtk::Label,
    overlay: Cell<bool>,
}

impl TeleprompterPage {
    pub fn new(state: &Rc<StudioState>) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(20);
        root.set_margin_end(20);

        let heading = gtk::Label::new(Some("Teleprompter"));
        heading.add_css_class("title-1");
        heading.set_halign(gtk::Align::Start);
        let note = gtk::Label::new(Some(
            "Usable while recording. Shortcuts stay in-app — COSMIC/Wayland has no safe rootless global hotkey API here. Overlay is a second window, not a compositor HUD.",
        ));
        note.add_css_class("caption");
        note.set_wrap(true);
        note.set_halign(gtk::Align::Start);

        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.set_text(&state.teleprompter.borrow().body);
        let view = gtk::TextView::with_buffer(&buffer);
        view.set_wrap_mode(gtk::WrapMode::Word);
        view.set_vexpand(true);
        view.set_top_margin(12);
        view.set_left_margin(12);
        view.set_right_margin(12);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_min_content_height(280);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&view));

        let script = state.teleprompter.borrow().clone();
        let font = gtk::Scale::with_range(gtk::Orientation::Horizontal, 18.0, 96.0, 1.0);
        font.set_value(script.font_size as f64);
        font.set_hexpand(true);
        let speed = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 8.0, 0.1);
        speed.set_value(script.scroll_speed as f64);
        speed.set_hexpand(true);
        let paused = gtk::CheckButton::with_label("Pause scroll");
        paused.set_active(script.paused);
        let mirrored = gtk::CheckButton::with_label("Mirrored");
        mirrored.set_active(script.mirrored);
        let overlay_btn = gtk::Button::with_label("Open overlay window");

        let controls = gtk::Box::new(gtk::Orientation::Vertical, 6);
        controls.append(&label_row("Font size", font.clone().upcast()));
        controls.append(&label_row("Scroll speed", speed.clone().upcast()));
        let toggles = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        toggles.append(&paused);
        toggles.append(&mirrored);
        toggles.append(&overlay_btn);
        controls.append(&toggles);

        let status = gtk::Label::new(Some("F5 toggles this page. Space pauses when this view is focused."));
        status.add_css_class("caption");
        status.set_halign(gtk::Align::Start);

        root.append(&heading);
        root.append(&note);
        root.append(&scroll);
        root.append(&controls);
        root.append(&status);

        let page = Rc::new(Self {
            root,
            view: view.clone(),
            scroll: scroll.clone(),
            paused: paused.clone(),
            mirrored: mirrored.clone(),
            status,
            overlay: Cell::new(false),
        });

        apply_font(&view, script.font_size);
        apply_mirror(&view, script.mirrored);

        let state_b = Rc::clone(state);
        buffer.connect_changed(move |buf| {
            let (start, end) = buf.bounds();
            state_b.teleprompter.borrow_mut().body = buf.text(&start, &end, false).to_string();
        });

        let state_f = Rc::clone(state);
        let view_f = view.clone();
        font.connect_value_changed(move |scale| {
            let size = clamp_font(scale.value() as f32);
            state_f.teleprompter.borrow_mut().font_size = size;
            apply_font(&view_f, size);
        });
        let state_s = Rc::clone(state);
        speed.connect_value_changed(move |scale| {
            state_s.teleprompter.borrow_mut().scroll_speed = clamp_speed(scale.value() as f32);
        });
        let state_p = Rc::clone(state);
        paused.connect_toggled(move |btn| {
            state_p.teleprompter.borrow_mut().paused = btn.is_active();
        });
        let state_m = Rc::clone(state);
        let view_m = view.clone();
        mirrored.connect_toggled(move |btn| {
            state_m.teleprompter.borrow_mut().mirrored = btn.is_active();
            apply_mirror(&view_m, btn.is_active());
        });

        let page_o = Rc::clone(&page);
        overlay_btn.connect_clicked(move |_| {
            page_o.open_overlay();
        });

        let page_t = Rc::clone(&page);
        let state_t = Rc::clone(state);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            page_t.tick(&state_t.teleprompter.borrow());
            glib::ControlFlow::Continue
        });

        page
    }

    fn tick(&self, script: &TeleprompterScript) {
        let adj = self.scroll.vadjustment();
        let delta = script.pixels_per_tick(0.05) as f64;
        if delta > 0.0 {
            let next = (adj.value() + delta).min(adj.upper() - adj.page_size());
            adj.set_value(next);
        }
        self.paused.set_active(script.paused);
    }

    pub fn toggle_pause(&self, state: &StudioState) {
        let paused = !state.teleprompter.borrow().paused;
        state.teleprompter.borrow_mut().paused = paused;
        self.paused.set_active(paused);
        self.status.set_text(if paused {
            "Scrolling paused."
        } else {
            "Scrolling."
        });
    }

    fn open_overlay(&self) {
        let win = gtk::Window::new();
        win.set_title(Some("Teleprompter overlay"));
        win.set_default_width(720);
        win.set_default_height(360);
        win.set_decorated(true);
        win.set_opacity(0.88);
        let label = gtk::Label::new(Some(
            "This is a second studio window, not a click-through compositor overlay. COSMIC/Wayland does not give this app a safe transparent HUD.",
        ));
        label.set_wrap(true);
        label.set_margin_top(16);
        label.set_margin_bottom(16);
        label.set_margin_start(16);
        label.set_margin_end(16);
        win.set_child(Some(&label));
        win.present();
        self.overlay.set(true);
        self.status.set_text("Opened an overlay window (not a system HUD).");
    }
}

fn apply_font(view: &gtk::TextView, size: f32) {
    view.add_css_class("scs-prompter");
    let css = format!("textview.scs-prompter {{ font-size: {size}px; }}");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn apply_mirror(view: &gtk::TextView, mirrored: bool) {
    if mirrored {
        view.add_css_class("scs-mirror");
    } else {
        view.remove_css_class("scs-mirror");
    }
}

fn label_row(title: &str, widget: gtk::Widget) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.add_css_class("caption");
    row.append(&label);
    row.append(&widget);
    row
}
