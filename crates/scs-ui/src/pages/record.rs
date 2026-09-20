use std::rc::Rc;

use gtk::prelude::*;
use scs_system::SystemSnapshot;

use crate::state::StudioState;
use crate::widgets::{dashboard, meters, mode_tiles, preview, status_strip};

pub struct RecordPage {
    pub root: gtk::ScrolledWindow,
    status: status_strip::StatusStrip,
    dashboard: dashboard::Dashboard,
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
        let preview = preview::build();
        let meters = meters::build();
        let status = status_strip::StatusStrip::build();

        let rec = gtk::Button::with_label("START RECORDING");
        rec.add_css_class("scs-rec-button");
        rec.add_css_class("destructive-action");
        rec.set_sensitive(false);
        rec.set_halign(gtk::Align::Center);
        rec.set_tooltip_text(Some("Available in a later milestone (M3)"));

        let rec_note = gtk::Label::new(Some(
            "Available in a later milestone — Milestone 1 never starts an encode.",
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
            "YouTube Live is designed here for later expansion. Unavailable in Milestone 1.",
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
        column.append(&preview);
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
        }
    }

    pub fn refresh(&self, state: &StudioState, snap: &SystemSnapshot) {
        self.status.refresh(state, snap);
        self.dashboard.refresh(state, snap);
    }
}
