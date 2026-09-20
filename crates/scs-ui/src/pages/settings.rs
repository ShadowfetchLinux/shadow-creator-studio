use std::rc::Rc;

use adw::prelude::*;
use scs_core::quality::QualityPreset;
use scs_core::settings::{AudioProcessingPreset, EncoderPreference};
use scs_encoder::listed_hardware;

use crate::state::StudioState;

pub fn build(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> gtk::ScrolledWindow {
    let page = adw::PreferencesPage::new();
    page.set_title("Settings");

    page.add(&general_group(state, window));
    page.add(&video_group(state));
    page.add(&audio_group(state));
    page.add(&camera_group(state));
    page.add(&recording_group(state, window));
    page.add(&streaming_group());
    page.add(&hotkeys_group());
    page.add(&advanced_group(state));
    page.add(&restore_group(state, window));

    let root = gtk::ScrolledWindow::new();
    root.set_child(Some(&page));
    root
}

fn general_group(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("General");
    group.set_description(Some("Appearance and first-run setup."));

    let theme = adw::ActionRow::builder()
        .title("Theme")
        .subtitle("Dark professional (forced)")
        .build();
    group.add(&theme);

    let wizard = adw::ActionRow::builder()
        .title("Open setup wizard")
        .subtitle("Re-run the first-run steps")
        .activatable(true)
        .build();
    let window = window.clone();
    let state = Rc::clone(state);
    wizard.connect_activated(move |_| {
        crate::wizard::present(&window, &state);
    });
    group.add(&wizard);
    group
}

fn video_group(state: &Rc<StudioState>) -> adw::PreferencesGroup {
    let settings = state.settings.borrow();
    let group = adw::PreferencesGroup::new();
    group.set_title("Video");
    group.set_description(Some("Canvas follows the recording quality preset."));

    let picture = adw::ActionRow::builder()
        .title("Output picture")
        .subtitle(format!(
            "{}×{} @ {} fps",
            settings.video.width, settings.video.height, settings.video.fps
        ))
        .build();
    group.add(&picture);
    group
}

fn audio_group(state: &Rc<StudioState>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Audio");
    group.set_description(Some(
        "Processing is modeled only. The chain is applied in Milestone 4. System mic config is never rewritten.",
    ));

    let model = gtk::StringList::new(&["Natural (light)", "Raw", "Voice", "Podcast"]);
    let combo = adw::ComboRow::builder()
        .title("Processing preset")
        .subtitle("Natural is the recommended default")
        .model(&model)
        .build();
    combo.set_selected(match state.settings.borrow().audio.processing_preset {
        AudioProcessingPreset::Natural => 0,
        AudioProcessingPreset::Raw => 1,
        AudioProcessingPreset::Voice => 2,
        AudioProcessingPreset::Podcast => 3,
    });
    let state_c = Rc::clone(state);
    combo.connect_selected_notify(move |row| {
        let preset = match row.selected() {
            1 => AudioProcessingPreset::Raw,
            2 => AudioProcessingPreset::Voice,
            3 => AudioProcessingPreset::Podcast,
            _ => AudioProcessingPreset::Natural,
        };
        state_c.settings.borrow_mut().audio.processing_preset = preset;
        let _ = state_c.persist();
    });
    group.add(&combo);

    let tracks = adw::SwitchRow::builder()
        .title("Separate mic and desktop tracks")
        .subtitle("Planned for recording. Saved now, used in M3+.")
        .active(state.settings.borrow().audio.separate_tracks)
        .build();
    let state_t = Rc::clone(state);
    tracks.connect_active_notify(move |row| {
        state_t.settings.borrow_mut().audio.separate_tracks = row.is_active();
        let _ = state_t.persist();
    });
    group.add(&tracks);

    let settings = state.settings.borrow();
    let devices = adw::ActionRow::builder()
        .title("Last microphone")
        .subtitle(
            settings
                .audio
                .mic_label
                .clone()
                .unwrap_or_else(|| "None selected yet — pick one on the Record page".into()),
        )
        .build();
    group.add(&devices);
    let desktop = adw::ActionRow::builder()
        .title("Last desktop audio")
        .subtitle(
            settings
                .audio
                .desktop_label
                .clone()
                .unwrap_or_else(|| "None selected yet — pick a monitor on the Record page".into()),
        )
        .build();
    group.add(&desktop);
    group
}

fn camera_group(state: &Rc<StudioState>) -> adw::PreferencesGroup {
    let settings = state.settings.borrow();
    let group = adw::PreferencesGroup::new();
    group.set_title("Camera");
    group.set_description(Some(
        "Choose the camera on the Record page. The last device and mirror option are saved.",
    ));
    let row = adw::ActionRow::builder()
        .title("Last camera")
        .subtitle(
            settings
                .camera
                .label
                .clone()
                .unwrap_or_else(|| "None selected yet — pick one on the Record page".into()),
        )
        .build();
    group.add(&row);
    let display = adw::ActionRow::builder()
        .title("Last display")
        .subtitle(
            settings
                .video
                .display_label
                .clone()
                .unwrap_or_else(|| "None selected yet".into()),
        )
        .build();
    group.add(&display);
    let mirror = adw::SwitchRow::builder()
        .title("Mirror preview")
        .subtitle("Horizontal flip for the live camera preview only. Not a recording setting yet.")
        .active(settings.camera.mirror_preview)
        .build();
    let state_m = Rc::clone(state);
    mirror.connect_active_notify(move |row| {
        state_m.settings.borrow_mut().camera.mirror_preview = row.is_active();
        let _ = state_m.persist();
    });
    group.add(&mirror);
    group
}

fn recording_group(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Recording");

    let project = adw::EntryRow::builder()
        .title("Project title")
        .text(state.settings.borrow().recording.project_title.as_str())
        .build();
    let state_p = Rc::clone(state);
    project.connect_changed(move |row| {
        state_p.settings.borrow_mut().recording.project_title = row.text().to_string();
        let _ = state_p.persist();
    });
    group.add(&project);

    let clip = adw::EntryRow::builder()
        .title("Clip title")
        .text(state.settings.borrow().recording.clip_title.as_str())
        .build();
    let state_c = Rc::clone(state);
    clip.connect_changed(move |row| {
        state_c.settings.borrow_mut().recording.clip_title = row.text().to_string();
        let _ = state_c.persist();
    });
    group.add(&clip);

    let labels: Vec<&str> = QualityPreset::ALL.iter().map(|q| q.label()).collect();
    let model = gtk::StringList::new(labels.as_slice());
    let quality = adw::ComboRow::builder()
        .title("Quality")
        .model(&model)
        .build();
    let current = state.settings.borrow().recording.quality;
    let idx = QualityPreset::ALL
        .iter()
        .position(|q| *q == current)
        .unwrap_or(0);
    quality.set_selected(idx as u32);
    let state_q = Rc::clone(state);
    quality.connect_selected_notify(move |row| {
        if let Some(preset) = QualityPreset::ALL.get(row.selected() as usize) {
            let mut settings = state_q.settings.borrow_mut();
            settings.recording.quality = *preset;
            settings.video.apply_quality(*preset);
            drop(settings);
            let _ = state_q.persist();
        }
    });
    group.add(&quality);

    let folder = adw::ActionRow::builder()
        .title("Recording folder")
        .subtitle(state.settings.borrow().recording.folder.display().to_string())
        .build();
    let browse = gtk::Button::with_label("Choose");
    browse.add_css_class("flat");
    let folder_label = folder.clone();
    let window = window.clone();
    let state_f = Rc::clone(state);
    browse.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Recording folder")
            .build();
        let state = Rc::clone(&state_f);
        let folder_label = folder_label.clone();
        dialog.select_folder(Some(&window), gtk::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    folder_label.set_subtitle(&path.display().to_string());
                    let _ = state.set_folder(path);
                }
            }
        });
    });
    folder.add_suffix(&browse);
    group.add(&folder);

    let remux = adw::SwitchRow::builder()
        .title("Remux to MP4 after recording")
        .subtitle("Copy remux after stop. The MKV is never deleted, even after the MP4 verifies.")
        .active(state.settings.borrow().recording.remux_to_mp4)
        .build();
    let state_r = Rc::clone(state);
    remux.connect_active_notify(move |row| {
        state_r.settings.borrow_mut().recording.remux_to_mp4 = row.is_active();
        let _ = state_r.persist();
    });
    group.add(&remux);
    group
}

fn streaming_group() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Streaming");
    group.set_description(Some("YouTube Live lands in Milestone 8. Keys stay out of this file."));

    let platform = adw::ActionRow::builder()
        .title("Platform")
        .subtitle("YouTube")
        .build();
    group.add(&platform);

    let key = adw::ActionRow::builder()
        .title("Stream key")
        .subtitle("Stored in the system keyring in a later milestone")
        .sensitive(false)
        .build();
    group.add(&key);
    group
}

fn hotkeys_group() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Hotkeys");
    group.set_description(Some("Shown as the planned defaults. Binding arrives later."));
    for (title, key) in [
        ("Start / stop", "F9"),
        ("Pause", "F10"),
        ("Marker", "F8"),
    ] {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(format!("{key} · Available in a later milestone"))
            .sensitive(false)
            .build();
        group.add(&row);
    }
    group
}

fn advanced_group(state: &Rc<StudioState>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Advanced");
    let engine = adw::ActionRow::builder()
        .title("Recording engine")
        .subtitle("FFmpeg in Milestone 3. OBS WebSocket is the later primary engine.")
        .build();
    group.add(&engine);

    let listed = listed_hardware(&state.caps);
    let hardware = if listed.is_empty() {
        "FFmpeg did not list NVENC on this machine.".into()
    } else {
        format!("Listed: {}", listed.join(", "))
    };
    let model = gtk::StringList::new(&[
        "Auto (NVENC if listed)",
        "NVENC H.264",
        "NVENC HEVC",
        "NVENC AV1",
        "Software libx264",
    ]);
    let encoder = adw::ComboRow::builder()
        .title("Encoder preference")
        .subtitle(hardware)
        .model(&model)
        .build();
    encoder.set_selected(match state.settings.borrow().advanced.encoder {
        EncoderPreference::AutoNvenc => 0,
        EncoderPreference::NvencH264 => 1,
        EncoderPreference::NvencHevc => 2,
        EncoderPreference::NvencAv1 => 3,
        EncoderPreference::SoftwareX264 => 4,
    });
    let state_e = Rc::clone(state);
    encoder.connect_selected_notify(move |row| {
        state_e.settings.borrow_mut().advanced.encoder = match row.selected() {
            1 => EncoderPreference::NvencH264,
            2 => EncoderPreference::NvencHevc,
            3 => EncoderPreference::NvencAv1,
            4 => EncoderPreference::SoftwareX264,
            _ => EncoderPreference::AutoNvenc,
        };
        let _ = state_e.persist();
    });
    group.add(&encoder);
    group
}

fn restore_group(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("Restore");
    let row = adw::ActionRow::builder()
        .title("Restore recommended settings")
        .subtitle("Reset to the YouTube-friendly defaults. Wizard completion is kept.")
        .activatable(true)
        .build();
    let state = Rc::clone(state);
    let window = window.clone();
    row.connect_activated(move |_| {
        if state.restore_recommended().is_ok() {
            let dialog = adw::AlertDialog::new(
                Some("Recommended settings restored"),
                Some("Defaults were written. Wizard completion was kept. Revisit this page to see every field refresh."),
            );
            dialog.add_response("ok", "OK");
            dialog.present(Some(&window));
        }
    });
    group.add(&row);
    group
}
