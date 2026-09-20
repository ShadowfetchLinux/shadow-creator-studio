use gtk::prelude::*;
use scs_core::disk::{format_bytes, format_duration_seconds};
use scs_system::SystemSnapshot;

use crate::state::StudioState;

pub struct StatusStrip {
    pub root: gtk::FlowBox,
    chips: Vec<gtk::Label>,
}

impl StatusStrip {
    pub fn build() -> Self {
        let root = gtk::FlowBox::new();
        root.set_selection_mode(gtk::SelectionMode::None);
        root.set_max_children_per_line(8);
        root.set_min_children_per_line(3);
        root.set_column_spacing(8);
        root.set_row_spacing(8);

        let labels = [
            "Status",
            "Timer",
            "Disk",
            "CPU",
            "GPU",
            "VRAM",
            "Encoder",
            "Picture",
            "Format",
            "Mic",
            "Camera",
        ];
        let mut chips = Vec::new();
        for label in labels {
            let chip = gtk::Label::new(Some(&format!("{label}: —")));
            chip.add_css_class("scs-status-chip");
            chip.set_halign(gtk::Align::Start);
            root.append(&chip);
            chips.push(chip);
        }

        Self { root, chips }
    }

    pub fn refresh(&self, state: &StudioState, snap: &SystemSnapshot) {
        let settings = state.settings.borrow();
        let quality = settings.recording.quality;
        let disk = snap
            .disk
            .map(|d| {
                let remain = d
                    .estimate_seconds(quality.estimated_bitrate_bps())
                    .map(format_duration_seconds)
                    .unwrap_or_else(|| "—".into());
                format!(
                    "Disk: {} free · ~{remain}",
                    format_bytes(d.available_bytes)
                )
            })
            .unwrap_or_else(|| "Disk: — · Unavailable".into());

        let cpu = snap
            .cpu_percent
            .map(|v| format!("CPU: {v:.0}%"))
            .unwrap_or_else(|| "CPU: — · Unavailable".into());
        let gpu = snap
            .gpu_util_percent
            .map(|v| format!("GPU: {v:.0}%"))
            .unwrap_or_else(|| "GPU: — · Unavailable".into());
        let vram = match (snap.vram_used_bytes, snap.vram_total_bytes) {
            (Some(used), Some(total)) => {
                format!("VRAM: {} / {}", format_bytes(used), format_bytes(total))
            }
            _ => "VRAM: — · Unavailable".into(),
        };
        let mic = settings
            .audio
            .mic_device
            .clone()
            .unwrap_or_else(|| "Unavailable — no device list yet".into());
        let camera = settings
            .camera
            .device
            .clone()
            .unwrap_or_else(|| "Unavailable — no device list yet".into());

        let values = [
            "Status: Idle · recording starts in Milestone 3".to_string(),
            "Timer: 00:00:00".into(),
            disk,
            cpu,
            gpu,
            vram,
            format!("Encoder: {}", state.encoder_status.borrow()),
            format!(
                "Picture: {}×{} @ {} fps",
                settings.video.width, settings.video.height, settings.video.fps
            ),
            format!("Format: {}", settings.recording.container.extension().to_uppercase()),
            format!("Mic: {mic}"),
            format!("Camera: {camera}"),
        ];
        for (chip, value) in self.chips.iter().zip(values) {
            chip.set_text(&value);
        }
    }
}
