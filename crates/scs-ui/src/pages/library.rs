use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use adw::prelude::*;
use scs_ffmpeg::{
    change_resolution, compress, extract_audio, normalize_audio, remux_mp4, thumbnail, to_gif,
    to_mp3, to_wav, trim, youtube_ready_mp4, ToolJob,
};
use scs_library::{confirm_delete, index_folder, rename_beside, DeleteRequest, LibraryEntry};

use crate::live::jobs::{self, JobEvent};
use crate::state::StudioState;

pub struct LibraryPage {
    pub root: gtk::ScrolledWindow,
    list: gtk::ListBox,
    detail: gtk::Label,
    status: gtk::Label,
    tool: gtk::DropDown,
    entries: RefCell<Vec<LibraryEntry>>,
    selected: Cell<i32>,
    window: adw::ApplicationWindow,
    job_rx: RefCell<Option<Receiver<JobEvent>>>,
    probes: RefCell<Vec<Receiver<JobEvent>>>,
}

impl LibraryPage {
    pub fn new(state: &Rc<StudioState>, window: &adw::ApplicationWindow) -> Rc<Self> {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
        column.set_margin_top(16);
        column.set_margin_bottom(20);
        column.set_margin_start(20);
        column.set_margin_end(20);

        let heading = gtk::Label::new(Some("Library"));
        heading.add_css_class("title-1");
        heading.set_halign(gtk::Align::Start);
        let note = gtk::Label::new(Some(
            "Indexed from the recordings folder only. Quick-edit tools write a new file (`-n`) and never overwrite the source.",
        ));
        note.add_css_class("caption");
        note.set_wrap(true);
        note.set_halign(gtk::Align::Start);

        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_min_content_height(260);
        scroll.set_child(Some(&list));

        let detail = gtk::Label::new(Some("Select a take."));
        detail.set_wrap(true);
        detail.set_halign(gtk::Align::Start);
        detail.set_selectable(true);

        let play = gtk::Button::with_label("Play");
        let folder_btn = gtk::Button::with_label("Open folder");
        let rename = gtk::Button::with_label("Rename");
        let remux = gtk::Button::with_label("Remux");
        let export = gtk::Button::with_label("Export");
        let copy = gtk::Button::with_label("Copy path");
        let delete = gtk::Button::with_label("Delete…");
        delete.add_css_class("destructive-action");
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for btn in [&play, &folder_btn, &rename, &remux, &export, &copy, &delete] {
            actions.append(btn);
        }

        let model = gtk::StringList::new(&[
            "Trim first 10s",
            "Normalize audio",
            "720p",
            "Compress",
            "Extract audio",
            "MP3",
            "WAV",
            "GIF (3s)",
            "Thumbnail",
            "YouTube-ready MP4",
        ]);
        let tool = gtk::DropDown::new(Some(model), gtk::Expression::NONE);
        tool.set_hexpand(true);
        let run = gtk::Button::with_label("Run tool");
        let tools = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        tools.append(&tool);
        tools.append(&run);

        let status = gtk::Label::new(Some(""));
        status.add_css_class("caption");
        status.set_wrap(true);
        status.set_halign(gtk::Align::Start);

        column.append(&heading);
        column.append(&note);
        column.append(&scroll);
        column.append(&detail);
        column.append(&actions);
        column.append(&tools);
        column.append(&status);

        let root = gtk::ScrolledWindow::new();
        root.set_child(Some(&column));

        let page = Rc::new(Self {
            root,
            list,
            detail,
            status,
            tool,
            entries: RefCell::new(Vec::new()),
            selected: Cell::new(-1),
            window: window.clone(),
            job_rx: RefCell::new(None),
            probes: RefCell::new(Vec::new()),
        });

        let page_sel = Rc::clone(&page);
        page.list.connect_row_selected(move |_, row| {
            page_sel.selected.set(row.map(|r| r.index()).unwrap_or(-1));
            page_sel.refresh_detail();
        });

        connect_action(&page, &play, Action::Play);
        connect_action(&page, &folder_btn, Action::OpenFolder);
        connect_action(&page, &rename, Action::Rename);
        connect_action(&page, &remux, Action::Remux);
        connect_action(&page, &export, Action::Export);
        connect_action(&page, &copy, Action::Copy);
        connect_action(&page, &delete, Action::Delete);

        let page_run = Rc::clone(&page);
        run.connect_clicked(move |_| {
            if let Err(err) = page_run.run_selected_tool() {
                page_run.status.set_text(&err);
            }
        });

        page.reload(state);
        page
    }

    pub fn reload(&self, state: &StudioState) {
        let folder = state.settings.borrow().recording.folder.clone();
        let index = index_folder(&folder);
        while let Some(row) = self.list.row_at_index(0) {
            self.list.remove(&row);
        }
        if index.entries.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No recordings in this folder")
                .subtitle(folder.display().to_string())
                .sensitive(false)
                .build();
            self.list.append(&row);
            self.status.set_text(
                index
                    .errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Record something and it will appear here.".into())
                    .as_str(),
            );
            self.entries.borrow_mut().clear();
            self.selected.set(-1);
            self.refresh_detail();
            return;
        }
        for entry in &index.entries {
            if entry.duration_secs.is_none() {
                self.probes
                    .borrow_mut()
                    .push(jobs::probe_and_write_sidecar(entry.path.clone()));
            }
            let subtitle = format!(
                "{} · {} · {} · {} · {} · {}",
                entry.modified.format("%Y-%m-%d %H:%M"),
                entry.duration_label(),
                entry.resolution_label(),
                entry
                    .fps
                    .map(|f| format!("{f:.0} fps"))
                    .unwrap_or_else(|| "FPS unknown".into()),
                entry
                    .video_codec
                    .clone()
                    .or(entry.audio_codec.clone())
                    .unwrap_or_else(|| "codec unknown".into()),
                entry.size_label()
            );
            let row = adw::ActionRow::builder()
                .title(entry.title.as_str())
                .subtitle(&subtitle)
                .activatable(true)
                .build();
            self.list.append(&row);
        }
        *self.entries.borrow_mut() = index.entries;
        if self.selected.get() < 0 {
            if let Some(row) = self.list.row_at_index(0) {
                self.list.select_row(Some(&row));
            }
        }
        self.status.set_text(&format!(
            "{} items in {}",
            self.entries.borrow().len(),
            folder.display()
        ));
        self.refresh_detail();
    }

    fn current(&self) -> Option<LibraryEntry> {
        let idx = self.selected.get();
        if idx < 0 {
            return None;
        }
        self.entries.borrow().get(idx as usize).cloned()
    }

    fn refresh_detail(&self) {
        let Some(entry) = self.current() else {
            self.detail.set_text("Select a take.");
            return;
        };
        self.detail.set_text(&format!(
            "{}\n{}\n{} · {} · {} · {}\nCodec: {} / {}\n{}",
            entry.title,
            entry.path.display(),
            entry.resolution_label(),
            entry
                .fps
                .map(|f| format!("{f:.0} fps"))
                .unwrap_or_else(|| "FPS unknown".into()),
            entry.duration_label(),
            entry.size_label(),
            entry.video_codec.as_deref().unwrap_or("—"),
            entry.audio_codec.as_deref().unwrap_or("—"),
            entry
                .thumbnail_path
                .as_ref()
                .map(|p| format!("Thumb: {}", p.display()))
                .unwrap_or_else(|| "No thumbnail yet — run the Thumbnail tool.".into())
        ));
    }

    fn run_selected_tool(&self) -> Result<(), String> {
        let entry = self.current().ok_or_else(|| "Select a take first.".to_string())?;
        let job = tool_for(&entry, self.tool.selected())?;
        *self.job_rx.borrow_mut() = Some(jobs::run_tool(job));
        self.status.set_text("Running FFmpeg tool…");
        Ok(())
    }

    fn apply_action(&self, action: Action) {
        let Some(entry) = self.current() else {
            self.status.set_text("Select a take first.");
            return;
        };
        match action {
            Action::Play => match jobs::open_path(&entry.path) {
                Ok(()) => self.status.set_text("Opened in the default player."),
                Err(err) => self.status.set_text(&err),
            },
            Action::OpenFolder => {
                let parent = entry.path.parent().map(|p| p.to_path_buf());
                match parent.as_deref().map(jobs::open_path) {
                    Some(Ok(())) => self.status.set_text("Opened the recordings folder."),
                    Some(Err(err)) => self.status.set_text(&err),
                    None => self.status.set_text("That file has no parent folder."),
                }
            }
            Action::Copy => {
                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&entry.path.display().to_string());
                    self.status.set_text("Path copied.");
                }
            }
            Action::Remux => {
                *self.job_rx.borrow_mut() = Some(jobs::run_tool(remux_mp4(&entry.path)));
                self.status.set_text("Remuxing to a new MP4…");
            }
            Action::Export => {
                *self.job_rx.borrow_mut() = Some(jobs::run_tool(youtube_ready_mp4(&entry.path)));
                self.status.set_text("Exporting a YouTube-ready MP4…");
            }
            Action::Rename => self.rename_dialog(&entry),
            Action::Delete => self.delete_dialog(&entry),
        }
    }

    fn rename_dialog(&self, entry: &LibraryEntry) {
        let dialog = adw::AlertDialog::new(Some("Rename"), Some("Writes a new filename beside the original. Does not overwrite."));
        let entry_row = adw::EntryRow::builder()
            .title("New name")
            .text(entry.title.as_str())
            .build();
        dialog.set_extra_child(Some(&entry_row));
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("rename", "Rename");
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        let src = entry.path.clone();
        let status = self.status.clone();
        dialog.connect_response(None, move |_, response| {
            if response != "rename" {
                return;
            }
            match rename_beside(&src, &entry_row.text()) {
                Ok(dest) => match std::fs::rename(&src, &dest) {
                    Ok(()) => {
                        let side = src.with_extension("json");
                        if side.exists() {
                            let _ = std::fs::rename(side, dest.with_extension("json"));
                        }
                        status.set_text(&format!("Renamed to {}", dest.display()));
                    }
                    Err(err) => status.set_text(&format!("Rename failed: {err}")),
                },
                Err(err) => status.set_text(&err),
            }
        });
        dialog.present(Some(&self.window));
    }

    fn delete_dialog(&self, entry: &LibraryEntry) {
        let dialog = adw::AlertDialog::new(
            Some("Delete this file?"),
            Some("This removes the media file. Sidecar JSON is removed too if present. This cannot be undone."),
        );
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("delete", "Delete");
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let path = entry.path.clone();
        let status = self.status.clone();
        dialog.connect_response(None, move |_, response| {
            if response != "delete" {
                return;
            }
            match confirm_delete(&DeleteRequest {
                path: path.clone(),
                confirmed: true,
            }) {
                Ok(path) => {
                    let side = path.with_extension("json");
                    let err = std::fs::remove_file(&path).err();
                    let _ = std::fs::remove_file(side);
                    if let Some(err) = err {
                        status.set_text(&format!("Could not delete: {err}"));
                    } else {
                        status.set_text("Deleted.");
                    }
                }
                Err(err) => status.set_text(&err),
            }
        });
        dialog.present(Some(&self.window));
    }

    pub fn pump(&self, state: &StudioState) {
        let mut reload = false;
        if let Some(rx) = self.job_rx.borrow().as_ref() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    JobEvent::Finished { message, .. } => {
                        self.status.set_text(&message);
                        reload = true;
                    }
                    JobEvent::Failed(err) => self.status.set_text(&err),
                }
            }
        }
        let mut probes = self.probes.borrow_mut();
        probes.retain(|rx| match rx.try_recv() {
            Ok(JobEvent::Finished { .. }) => {
                reload = true;
                false
            }
            Ok(JobEvent::Failed(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
        });
        drop(probes);
        if reload {
            self.reload(state);
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Play,
    OpenFolder,
    Rename,
    Remux,
    Export,
    Copy,
    Delete,
}

fn connect_action(page: &Rc<LibraryPage>, button: &gtk::Button, action: Action) {
    let page = Rc::clone(page);
    button.connect_clicked(move |_| page.apply_action(action));
}

pub fn tool_for(entry: &LibraryEntry, index: u32) -> Result<ToolJob, String> {
    let path = &entry.path;
    Ok(match index {
        0 => trim(path, 0.0, 10.0),
        1 => normalize_audio(path),
        2 => change_resolution(path, 1280, 720),
        3 => compress(path, 26),
        4 => extract_audio(path),
        5 => to_mp3(path),
        6 => to_wav(path),
        7 => to_gif(path, 0.0, 3.0),
        8 => thumbnail(path, 1.0),
        9 => youtube_ready_mp4(path),
        _ => remux_mp4(path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tool_builders_keep_source_path() {
        let entry = LibraryEntry::from_path(Path::new("/tmp/title;rm.mkv"), None, None);
        let job = tool_for(&entry, 5).unwrap();
        assert!(job.output.extension().unwrap() == "mp3");
        assert!(job
            .command
            .args
            .iter()
            .any(|a| a.to_string_lossy() == "/tmp/title;rm.mkv"));
    }
}
