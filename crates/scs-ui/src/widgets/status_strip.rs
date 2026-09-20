use gtk::prelude::*;
use scs_core::disk::{format_bytes, format_clock, format_duration_seconds};
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
            .mic_label
            .clone()
            .or_else(|| settings.audio.mic_device.clone())
            .unwrap_or_else(|| "Unavailable — no microphone".into());
        let camera = settings
            .camera
            .label
            .clone()
            .or_else(|| settings.camera.device.clone())
            .unwrap_or_else(|| "Unavailable — no camera".into());

        let rec = state.recording.borrow();
        let (status, timer, encoder) = if rec.active {
            let elapsed = rec
                .started
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            let warn = rec
                .warning
                .as_deref()
                .map(|w| format!(" · {w}"))
                .unwrap_or_default();
            let drop = rec
                .dropped
                .map(|n| format!(" · dropped {n}"))
                .unwrap_or_default();
            (
                format!("Status: Recording{warn}{drop}"),
                format!("Timer: {}", format_clock(elapsed)),
                if rec.encoder.is_empty() {
                    format!("Encoder: {}", state.encoder_status.borrow())
                } else {
                    format!("Encoder: {} · recording", rec.encoder)
                },
            )
        } else {
            (
                rec.last_message
                    .clone()
                    .unwrap_or_else(|| "Status: Idle".into()),
                "Timer: 00:00:00".into(),
                format!("Encoder: {}", state.encoder_status.borrow()),
            )
        };
        drop(rec);

        let values = [
            status,
            timer,
            disk,
            cpu,
            gpu,
            vram,
            encoder,
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
