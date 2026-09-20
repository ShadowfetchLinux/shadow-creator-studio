use std::rc::Rc;

use gtk::prelude::*;
use scs_audio::{looks_like_headphone, monitor_policy};
use scs_core::settings::AudioProcessingPreset;

use crate::state::StudioState;

pub struct TrackMixer {
    pub root: gtk::Box,
    pub calibrate: gtk::Button,
    pub advice: gtk::Label,
}

impl TrackMixer {
    pub fn build(state: &Rc<StudioState>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("scs-card");

        let title = gtk::Label::new(Some("Tracks and processing"));
        title.add_css_class("heading");
        title.set_halign(gtk::Align::Start);
        root.append(&title);

        let note = gtk::Label::new(Some(
            "Track 1 mixed · 2 mic · 3 desktop · 4 optional music. Mute and volume stay in the FFmpeg graph. System mic settings are never rewritten.",
        ));
        note.add_css_class("caption");
        note.set_wrap(true);
        note.set_halign(gtk::Align::Start);
        root.append(&note);

        let labels: Vec<&str> = AudioProcessingPreset::ALL.iter().map(|p| p.label()).collect();
        let model = gtk::StringList::new(&labels);
        let combo = gtk::DropDown::new(Some(model), gtk::Expression::NONE);
        let current = state.settings.borrow().audio.processing_preset;
        let idx = AudioProcessingPreset::ALL
            .iter()
            .position(|p| *p == current)
            .unwrap_or(0);
        combo.set_selected(idx as u32);
        let state_p = Rc::clone(state);
        combo.connect_selected_notify(move |row| {
            if let Some(preset) = AudioProcessingPreset::ALL.get(row.selected() as usize) {
                state_p.settings.borrow_mut().audio.processing_preset = *preset;
                let _ = state_p.persist();
            }
        });
        root.append(&labeled("Preset", combo.upcast()));

        root.append(&bus_row(state, "Mic", BusKind::Mic));
        root.append(&bus_row(state, "Desktop", BusKind::Desktop));
        root.append(&bus_row(state, "Music", BusKind::Music));

        let mixed = gtk::CheckButton::with_label("Write mixed track");
        mixed.set_active(state.settings.borrow().audio.include_mixed);
        let state_m = Rc::clone(state);
        mixed.connect_toggled(move |btn| {
            state_m.settings.borrow_mut().audio.include_mixed = btn.is_active();
            let _ = state_m.persist();
        });
        root.append(&mixed);

        let separate = gtk::CheckButton::with_label("Write separate source tracks");
        separate.set_active(state.settings.borrow().audio.separate_tracks);
        let state_s = Rc::clone(state);
        separate.connect_toggled(move |btn| {
            state_s.settings.borrow_mut().audio.separate_tracks = btn.is_active();
            let _ = state_s.persist();
        });
        root.append(&separate);

        let headphones = state
            .settings
            .borrow()
            .audio
            .desktop_label
            .as_deref()
            .map(looks_like_headphone)
            .unwrap_or(false);
        let policy = monitor_policy(state.settings.borrow().audio.monitor_enabled, headphones);
        let monitor = gtk::CheckButton::with_label("Mic monitoring (no speaker loop)");
        monitor.set_active(policy.enabled_pref);
        let state_mon = Rc::clone(state);
        monitor.connect_toggled(move |btn| {
            state_mon.settings.borrow_mut().audio.monitor_enabled = btn.is_active();
            let _ = state_mon.persist();
        });
        root.append(&monitor);
        let mon_note = gtk::Label::new(Some(policy.reason));
        mon_note.add_css_class("scs-unavailable");
        mon_note.set_wrap(true);
        mon_note.set_halign(gtk::Align::Start);
        root.append(&mon_note);

        let calibrate = gtk::Button::with_label("Calibrate mic…");
        calibrate.set_halign(gtk::Align::Start);
        root.append(&calibrate);

        let advice = gtk::Label::new(Some("Speak a few seconds, then calibrate. Peak and average meters stay live."));
        advice.add_css_class("caption");
        advice.set_wrap(true);
        advice.set_halign(gtk::Align::Start);
        root.append(&advice);

        Self {
            root,
            calibrate,
            advice,
        }
    }

    pub fn show_advice(&self, text: &str) {
        self.advice.set_text(text);
    }
}

#[derive(Clone, Copy)]
enum BusKind {
    Mic,
    Desktop,
    Music,
}

fn bus_row(state: &Rc<StudioState>, title: &str, kind: BusKind) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(title));
    label.set_width_chars(8);
    label.set_halign(gtk::Align::Start);

    let (muted, volume, include) = {
        let a = &state.settings.borrow().audio;
        match kind {
            BusKind::Mic => (a.mic_muted, a.mic_volume, a.include_mic),
            BusKind::Desktop => (a.desktop_muted, a.desktop_volume, a.include_desktop),
            BusKind::Music => (a.music_muted, a.music_volume, a.include_music),
        }
    };

    let mute = gtk::CheckButton::with_label("Mute");
    mute.set_active(muted);
    let state_mu = Rc::clone(state);
    mute.connect_toggled(move |btn| {
        let active = btn.is_active();
        let mut s = state_mu.settings.borrow_mut();
        match kind {
            BusKind::Mic => s.audio.mic_muted = active,
            BusKind::Desktop => s.audio.desktop_muted = active,
            BusKind::Music => s.audio.music_muted = active,
        }
        drop(s);
        let _ = state_mu.persist();
    });

    let on = gtk::CheckButton::with_label("Track");
    on.set_active(include);
    let state_on = Rc::clone(state);
    on.connect_toggled(move |btn| {
        let active = btn.is_active();
        let mut s = state_on.settings.borrow_mut();
        match kind {
            BusKind::Mic => s.audio.include_mic = active,
            BusKind::Desktop => s.audio.include_desktop = active,
            BusKind::Music => s.audio.include_music = active,
        }
        drop(s);
        let _ = state_on.persist();
    });

    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 2.2, 0.05);
    scale.set_value(volume as f64);
    scale.set_hexpand(true);
    scale.set_width_request(140);
    let state_v = Rc::clone(state);
    scale.connect_value_changed(move |scale| {
        let value = scale.value() as f32;
        let mut s = state_v.settings.borrow_mut();
        match kind {
            BusKind::Mic => s.audio.mic_volume = value,
            BusKind::Desktop => s.audio.desktop_volume = value,
            BusKind::Music => s.audio.music_volume = value,
        }
        drop(s);
        let _ = state_v.persist();
    });

    row.append(&label);
    row.append(&on);
    row.append(&mute);
    row.append(&scale);
    row
}

fn labeled(title: &str, widget: gtk::Widget) -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.add_css_class("caption");
    box_.append(&label);
    box_.append(&widget);
    box_
}
