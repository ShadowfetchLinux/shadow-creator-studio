use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use scs_audio::AudioDevice;
use scs_capture::{CameraDevice, DesktopOption, DisplaySource};

use crate::state::StudioState;

pub struct SourceSelector {
    pub root: gtk::Box,
    cameras: gtk::Box,
    displays: gtk::Box,
    mic: gtk::DropDown,
    desktop: gtk::DropDown,
    camera_ids: Rc<RefCell<Vec<String>>>,
    display_ids: Rc<RefCell<Vec<String>>>,
    mic_ids: Rc<RefCell<Vec<String>>>,
    desk_ids: Rc<RefCell<Vec<String>>>,
    suppress: Rc<Cell<bool>>,
}

impl SourceSelector {
    pub fn build(state: &Rc<StudioState>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);

        let cameras = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        cameras.add_css_class("scs-source-row");
        let displays = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        displays.add_css_class("scs-source-row");

        let extra = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        extra.append(&unavailable_card(
            "Window",
            DesktopOption::window_unavailable()
                .reason
                .as_deref()
                .unwrap_or(""),
        ));
        extra.append(&unavailable_card(
            "Region",
            DesktopOption::region_unavailable()
                .reason
                .as_deref()
                .unwrap_or(""),
        ));

        let (mic_wrap, mic, mic_ids) = labeled_dropdown("Microphone");
        let (desk_wrap, desktop, desk_ids) = labeled_dropdown("Desktop audio");
        let audio_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        audio_row.set_homogeneous(true);
        audio_row.append(&mic_wrap);
        audio_row.append(&desk_wrap);

        let suppress = Rc::new(Cell::new(false));
        bind_audio_dropdown(
            &mic,
            &mic_ids,
            Rc::clone(state),
            AudioKind::Mic,
            Rc::clone(&suppress),
        );
        bind_audio_dropdown(
            &desktop,
            &desk_ids,
            Rc::clone(state),
            AudioKind::Desktop,
            Rc::clone(&suppress),
        );

        root.append(&heading("Camera"));
        root.append(&cameras);
        root.append(&heading("Display / monitor"));
        root.append(&displays);
        root.append(&extra);
        root.append(&heading("Audio"));
        root.append(&audio_row);

        Self {
            root,
            cameras,
            displays,
            mic,
            desktop,
            camera_ids: Rc::new(RefCell::new(Vec::new())),
            display_ids: Rc::new(RefCell::new(Vec::new())),
            mic_ids,
            desk_ids,
            suppress,
        }
    }

    pub fn set_cameras(&self, cameras: &[CameraDevice], state: &Rc<StudioState>) {
        while let Some(child) = self.cameras.first_child() {
            self.cameras.remove(&child);
        }
        self.camera_ids.borrow_mut().clear();
        if cameras.is_empty() {
            self.cameras
                .append(&unavailable_card("Camera", "No camera reported a usable format."));
            return;
        }
        let selected = state.settings.borrow().camera.device.clone();
        let mut group: Option<gtk::ToggleButton> = None;
        for camera in cameras {
            let button = gtk::ToggleButton::new();
            button.add_css_class("scs-mode-tile");
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let title = gtk::Label::new(Some(&camera.name));
            title.add_css_class("scs-mode-title");
            let sub = gtk::Label::new(Some(
                &camera
                    .preferred_format()
                    .map(|fmt| fmt.label())
                    .unwrap_or_else(|| camera.path.clone()),
            ));
            sub.add_css_class("caption");
            inner.append(&title);
            inner.append(&sub);
            button.set_child(Some(&inner));
            if let Some(ref lead) = group {
                button.set_group(Some(lead));
            } else {
                group = Some(button.clone());
            }
            if selected.as_deref() == Some(camera.path.as_str()) {
                button.set_active(true);
            }
            let path = camera.path.clone();
            let name = camera.name.clone();
            let fmt = camera.preferred_format().cloned();
            let state = Rc::clone(state);
            button.connect_toggled(move |btn| {
                if btn.is_active() {
                    let mut settings = state.settings.borrow_mut();
                    settings.camera.device = Some(path.clone());
                    settings.camera.label = Some(name.clone());
                    if let Some(fmt) = &fmt {
                        settings.camera.width = fmt.width;
                        settings.camera.height = fmt.height;
                        settings.camera.pixel_format = Some(fmt.pixel_format.clone());
                    }
                    drop(settings);
                    let _ = state.persist();
                }
            });
            self.camera_ids.borrow_mut().push(camera.path.clone());
            self.cameras.append(&button);
        }
        if selected.is_none() {
            if let Some(first) = self.cameras.first_child() {
                if let Ok(btn) = first.downcast::<gtk::ToggleButton>() {
                    btn.set_active(true);
                }
            }
        }
    }

    pub fn set_displays(&self, displays: &[DisplaySource], state: &Rc<StudioState>) {
        while let Some(child) = self.displays.first_child() {
            self.displays.remove(&child);
        }
        self.display_ids.borrow_mut().clear();
        if displays.is_empty() {
            self.displays.append(&unavailable_card(
                "Display",
                "No monitors were reported by the session.",
            ));
            return;
        }
        let selected = state.settings.borrow().video.display_id.clone();
        let mut group: Option<gtk::ToggleButton> = None;
        for display in displays {
            let button = gtk::ToggleButton::new();
            button.add_css_class("scs-mode-tile");
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let title = gtk::Label::new(Some(&display.label));
            title.add_css_class("scs-mode-title");
            let sub = gtk::Label::new(Some(&format!(
                "{}×{}{}",
                display.width,
                display.height,
                if display.primary { " · primary" } else { "" }
            )));
            sub.add_css_class("caption");
            inner.append(&title);
            inner.append(&sub);
            button.set_child(Some(&inner));
            if let Some(ref lead) = group {
                button.set_group(Some(lead));
            } else {
                group = Some(button.clone());
            }
            if selected.as_deref() == Some(display.id.as_str()) {
                button.set_active(true);
            }
            let id = display.id.clone();
            let label = display.geometry_label();
            let state = Rc::clone(state);
            button.connect_toggled(move |btn| {
                if btn.is_active() {
                    state.settings.borrow_mut().video.display_id = Some(id.clone());
                    state.settings.borrow_mut().video.display_label = Some(label.clone());
                    let _ = state.persist();
                }
            });
            self.display_ids.borrow_mut().push(display.id.clone());
            self.displays.append(&button);
        }
        if selected.is_none() {
            if let Some(first) = self.displays.first_child() {
                if let Ok(btn) = first.downcast::<gtk::ToggleButton>() {
                    btn.set_active(true);
                }
            }
        }
    }

    pub fn set_audio(
        &self,
        mics: &[AudioDevice],
        desktop: &[AudioDevice],
        state: &Rc<StudioState>,
    ) {
        let mic_sel = state.settings.borrow().audio.mic_device.clone();
        let desk_sel = state.settings.borrow().audio.desktop_device.clone();
        fill_dropdown(
            &self.mic,
            &self.mic_ids,
            mics,
            mic_sel.as_deref(),
            &self.suppress,
        );
        fill_dropdown(
            &self.desktop,
            &self.desk_ids,
            desktop,
            desk_sel.as_deref(),
            &self.suppress,
        );
    }
}

#[derive(Clone, Copy)]
enum AudioKind {
    Mic,
    Desktop,
}

fn bind_audio_dropdown(
    drop: &gtk::DropDown,
    ids: &Rc<RefCell<Vec<String>>>,
    state: Rc<StudioState>,
    kind: AudioKind,
    suppress: Rc<Cell<bool>>,
) {
    let ids = Rc::clone(ids);
    drop.connect_selected_notify(move |row| {
        if suppress.get() {
            return;
        }
        let idx = row.selected() as usize;
        let id = ids.borrow().get(idx).cloned();
        let Some(id) = id else { return };
        if id.is_empty() {
            return;
        }
        let label = row
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .map(|obj| obj.string().to_string());
        {
            let mut settings = state.settings.borrow_mut();
            match kind {
                AudioKind::Mic => {
                    settings.audio.mic_device = Some(id);
                    settings.audio.mic_label = label;
                }
                AudioKind::Desktop => {
                    settings.audio.desktop_device = Some(id);
                    settings.audio.desktop_label = label;
                }
            }
        }
        let _ = state.persist();
    });
}

fn fill_dropdown(
    drop: &gtk::DropDown,
    ids: &Rc<RefCell<Vec<String>>>,
    devices: &[AudioDevice],
    selected: Option<&str>,
    suppress: &Cell<bool>,
) {
    let mut labels = Vec::new();
    let mut stored = Vec::new();
    if devices.is_empty() {
        labels.push("Unavailable — no device found");
        stored.push(String::new());
    } else {
        for device in devices {
            labels.push(device.name.as_str());
            stored.push(device.id.clone());
        }
    }
    suppress.set(true);
    let model = gtk::StringList::new(&labels);
    drop.set_model(Some(&model));
    let select = selected
        .and_then(|id| stored.iter().position(|item| item == id))
        .unwrap_or(0);
    *ids.borrow_mut() = stored;
    drop.set_selected(select as u32);
    suppress.set(false);
}

fn labeled_dropdown(title: &str) -> (gtk::Box, gtk::DropDown, Rc<RefCell<Vec<String>>>) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
    root.add_css_class("scs-card");
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.add_css_class("heading");
    let model = gtk::StringList::new(&["Unavailable — scanning…"]);
    let drop = gtk::DropDown::new(Some(model), gtk::Expression::NONE);
    drop.set_hexpand(true);
    root.append(&label);
    root.append(&drop);
    (root, drop, Rc::new(RefCell::new(vec![String::new()])))
}

fn heading(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("heading");
    label.set_halign(gtk::Align::Start);
    label
}

fn unavailable_card(title: &str, reason: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.add_css_class("scs-card");
    card.add_css_class("scs-source-tile");
    card.set_sensitive(false);
    let t = gtk::Label::new(Some(title));
    t.add_css_class("scs-mode-title");
    t.set_halign(gtk::Align::Start);
    let s = gtk::Label::new(Some(reason));
    s.add_css_class("scs-unavailable");
    s.add_css_class("caption");
    s.set_wrap(true);
    s.set_halign(gtk::Align::Start);
    card.append(&t);
    card.append(&s);
    card
}
