use gtk::prelude::*;

pub struct MeterPair {
    pub root: gtk::Box,
}

pub fn build() -> MeterPair {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    root.set_homogeneous(true);
    root.append(&meter_card(
        "Microphone",
        "Idle — meters arrive in Milestone 2",
    ));
    root.append(&meter_card(
        "Desktop audio",
        "Idle — meters arrive in Milestone 2",
    ));
    MeterPair { root }
}

fn meter_card(title: &str, note: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.add_css_class("scs-card");

    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.add_css_class("heading");

    let track = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    track.add_css_class("scs-meter-track");
    track.set_hexpand(true);

    let bar = gtk::LevelBar::new();
    bar.set_value(0.0);
    bar.set_min_value(0.0);
    bar.set_max_value(1.0);
    bar.set_sensitive(false);
    bar.set_hexpand(true);

    let status = gtk::Label::new(Some(note));
    status.set_halign(gtk::Align::Start);
    status.add_css_class("scs-unavailable");
    status.add_css_class("caption");

    card.append(&label);
    card.append(&track);
    card.append(&bar);
    card.append(&status);
    card
}
