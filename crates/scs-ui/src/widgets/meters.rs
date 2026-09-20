use gtk::prelude::*;
use scs_audio::{meter_fraction, MeterLevels};

pub struct LiveMeter {
    pub root: gtk::Box,
    bar: gtk::LevelBar,
    avg: gtk::LevelBar,
    status: gtk::Label,
    clip: gtk::Label,
}

impl LiveMeter {
    pub fn build(title: &str) -> Self {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        card.add_css_class("scs-card");

        let label = gtk::Label::new(Some(title));
        label.set_halign(gtk::Align::Start);
        label.add_css_class("heading");

        let bar = gtk::LevelBar::new();
        bar.set_min_value(0.0);
        bar.set_max_value(1.0);
        bar.set_value(0.0);
        bar.set_hexpand(true);
        bar.add_css_class("scs-meter-bar");

        let avg = gtk::LevelBar::new();
        avg.set_min_value(0.0);
        avg.set_max_value(1.0);
        avg.set_value(0.0);
        avg.set_hexpand(true);

        let status = gtk::Label::new(Some("Waiting for device…"));
        status.set_halign(gtk::Align::Start);
        status.add_css_class("caption");

        let clip = gtk::Label::new(Some(""));
        clip.set_halign(gtk::Align::Start);
        clip.add_css_class("scs-clip");

        card.append(&label);
        card.append(&gtk::Label::builder().label("Peak").css_classes(["caption"]).halign(gtk::Align::Start).build());
        card.append(&bar);
        card.append(&gtk::Label::builder().label("Average").css_classes(["caption"]).halign(gtk::Align::Start).build());
        card.append(&avg);
        card.append(&status);
        card.append(&clip);

        Self {
            root: card,
            bar,
            avg,
            status,
            clip,
        }
    }

    pub fn set_idle(&self, message: &str) {
        self.bar.set_value(0.0);
        self.avg.set_value(0.0);
        self.status.set_text(message);
        self.status.add_css_class("scs-unavailable");
        self.clip.set_text("");
        self.clip.remove_css_class("scs-clip-hot");
    }

    pub fn set_error(&self, message: &str) {
        self.set_idle(message);
    }

    pub fn set_levels(&self, levels: MeterLevels, device_label: &str) {
        self.bar.set_value(meter_fraction(levels.peak_db));
        self.avg.set_value(meter_fraction(levels.average_db));
        self.status.remove_css_class("scs-unavailable");
        self.status.set_text(&format!(
            "{device_label}  ·  peak {:+.1} dB  ·  avg {:+.1} dB",
            levels.peak_db, levels.average_db
        ));
        if levels.clipping {
            self.clip.set_text("Clipping");
            self.clip.add_css_class("scs-clip-hot");
        } else {
            self.clip.set_text("");
            self.clip.remove_css_class("scs-clip-hot");
        }
    }
}

pub struct MeterPair {
    pub root: gtk::Box,
    pub mic: LiveMeter,
    pub desktop: LiveMeter,
}

pub fn build() -> MeterPair {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    root.set_homogeneous(true);
    let mic = LiveMeter::build("Microphone");
    let desktop = LiveMeter::build("Desktop audio");
    root.append(&mic.root);
    root.append(&desktop.root);
    MeterPair {
        root,
        mic,
        desktop,
    }
}
