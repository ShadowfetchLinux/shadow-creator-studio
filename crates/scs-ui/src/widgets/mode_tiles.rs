use std::rc::Rc;

use gtk::prelude::*;
use scs_core::RecordingMode;

use crate::state::StudioState;

pub fn build(state: &Rc<StudioState>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_homogeneous(true);
    let current = state.settings.borrow().last_recording_mode;
    let mut group: Option<gtk::ToggleButton> = None;

    for mode in RecordingMode::ALL {
        let button = gtk::ToggleButton::new();
        button.add_css_class("scs-mode-tile");
        button.set_hexpand(true);

        let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
        inner.set_halign(gtk::Align::Center);
        inner.set_valign(gtk::Align::Center);
        let title = gtk::Label::new(Some(mode.label()));
        title.add_css_class("scs-mode-title");
        let sub = gtk::Label::new(Some(mode.description()));
        sub.add_css_class("caption");
        sub.add_css_class("dim-label");
        inner.append(&title);
        inner.append(&sub);
        button.set_child(Some(&inner));

        if let Some(ref lead) = group {
            button.set_group(Some(lead));
        } else {
            group = Some(button.clone());
        }
        if mode == current {
            button.set_active(true);
        }

        let state = Rc::clone(state);
        button.connect_toggled(move |btn| {
            if btn.is_active() {
                let _ = state.select_mode(mode);
            }
        });
        row.append(&button);
    }

    row
}
