use gtk::prelude::*;
use scs_core::disk::{format_bytes, format_duration_seconds};
use scs_system::SystemSnapshot;

use crate::state::StudioState;

pub struct Dashboard {
    pub root: gtk::FlowBox,
    values: Vec<gtk::Label>,
}

impl Dashboard {
    pub fn build() -> Self {
        let root = gtk::FlowBox::new();
        root.set_selection_mode(gtk::SelectionMode::None);
        root.set_max_children_per_line(6);
        root.set_min_children_per_line(3);
        root.set_column_spacing(8);
        root.set_row_spacing(8);

        let titles = [
            "CPU",
            "RAM",
            "GPU",
            "VRAM",
            "GPU temp",
            "CPU temp",
            "Bitrate",
            "Dropped",
            "Encode FPS",
            "Disk write",
            "Free space",
            "Est. time",
        ];
        let mut values = Vec::new();
        for title in titles {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 2);
            card.add_css_class("scs-card");
            let label = gtk::Label::new(Some(title));
            label.add_css_class("scs-metric-label");
            label.set_halign(gtk::Align::Start);
            let value = gtk::Label::new(Some("—"));
            value.add_css_class("scs-metric-value");
            value.set_halign(gtk::Align::Start);
            card.append(&label);
            card.append(&value);
            root.append(&card);
            values.push(value);
        }
        Self { root, values }
    }

    pub fn refresh(&self, state: &StudioState, snap: &SystemSnapshot) {
        let settings = state.settings.borrow();
        let bitrate = settings.recording.quality.estimated_bitrate_bps();
        let pairs = [
            opt_pct(snap.cpu_percent),
            ram(snap.ram_used_bytes, snap.ram_total_bytes),
            opt_pct(snap.gpu_util_percent),
            ram(snap.vram_used_bytes, snap.vram_total_bytes),
            opt_temp(snap.gpu_temp_c),
            opt_temp(snap.cpu_temp_c),
            unavailable("not measured until recording"),
            unavailable("not measured until recording"),
            unavailable("not measured until recording"),
            unavailable("not measured until recording"),
            snap.disk
                .map(|d| format_bytes(d.available_bytes))
                .map(avail)
                .unwrap_or_else(|| unavailable("disk probe failed")),
            snap.disk
                .and_then(|d| d.estimate_seconds(bitrate))
                .map(format_duration_seconds)
                .map(avail)
                .unwrap_or_else(|| unavailable("need free space + bitrate")),
        ];
        for (label, (text, live)) in self.values.iter().zip(pairs) {
            label.set_text(&text);
            if live {
                label.remove_css_class("scs-unavailable");
            } else {
                label.add_css_class("scs-unavailable");
            }
        }
    }
}

fn avail(text: String) -> (String, bool) {
    (text, true)
}

fn unavailable(reason: &str) -> (String, bool) {
    (format!("— · {reason}"), false)
}

fn opt_pct(value: Option<f32>) -> (String, bool) {
    value
        .map(|v| (format!("{v:.0}%"), true))
        .unwrap_or_else(|| unavailable("not sampled"))
}

fn opt_temp(value: Option<f32>) -> (String, bool) {
    value
        .map(|v| (format!("{v:.0} °C"), true))
        .unwrap_or_else(|| unavailable("sensor missing"))
}

fn ram(used: Option<u64>, total: Option<u64>) -> (String, bool) {
    match (used, total) {
        (Some(u), Some(t)) => (format!("{} / {}", format_bytes(u), format_bytes(t)), true),
        _ => unavailable("not sampled"),
    }
}
