use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::entry::LibraryEntry;
use crate::sidecar::{load_sidecar, sidecar_path};

const MEDIA_EXT: &[&str] = &["mkv", "mp4", "webm", "mov", "m4a", "wav", "mp3"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub folder: PathBuf,
    pub entries: Vec<LibraryEntry>,
    pub errors: Vec<String>,
}

impl LibraryIndex {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Index only the configured recordings folder (non-recursive besides one level).
pub fn index_folder(folder: impl AsRef<Path>) -> LibraryIndex {
    let folder = folder.as_ref().to_path_buf();
    let mut index = LibraryIndex {
        folder: folder.clone(),
        entries: Vec::new(),
        errors: Vec::new(),
    };
    if !folder.exists() {
        index
            .errors
            .push("The recordings folder does not exist yet.".into());
        return index;
    }
    let read = match fs::read_dir(&folder) {
        Ok(rd) => rd,
        Err(err) => {
            index.errors.push(format!("Could not read the folder: {err}"));
            return index;
        }
    };
    for item in read.flatten() {
        let path = item.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !MEDIA_EXT.iter().any(|ok| ext.eq_ignore_ascii_case(ok)) {
            continue;
        }
        if path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with("_thumb") || s.ends_with("_audio") || s.ends_with("_clip"))
            .unwrap_or(false)
            && matches!(ext, "jpg" | "png" | "gif")
        {
            continue;
        }
        let meta = fs::metadata(&path).ok();
        let sidecar = load_sidecar(&sidecar_path(&path));
        let mut entry = LibraryEntry::from_path(&path, meta.as_ref(), sidecar.as_ref());
        if let Some(thumb) = sibling_thumb(&path) {
            entry.thumbnail_path = Some(thumb);
        }
        index.entries.push(entry);
    }
    index.entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    index
}

fn sibling_thumb(media: &Path) -> Option<PathBuf> {
    let stem = media.file_stem()?.to_string_lossy();
    let jpg = media.with_file_name(format!("{stem}_thumb.jpg"));
    if jpg.exists() {
        Some(jpg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn indexes_only_media_in_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("take.mkv"), b"not-really-media").unwrap();
        fs::write(dir.path().join("notes.txt"), b"skip").unwrap();
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested").join("hidden.mkv"), b"no").unwrap();
        let index = index_folder(dir.path());
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].title, "take");
        assert!(index.entries[0].path.ends_with("take.mkv"));
    }

    #[test]
    fn missing_folder_is_empty_with_reason() {
        let index = index_folder("/tmp/scs-does-not-exist-folder");
        assert!(index.is_empty());
        assert!(!index.errors.is_empty());
    }

    #[test]
    fn sidecar_fills_title() {
        let dir = tempfile::tempdir().unwrap();
        let mkv = dir.path().join("clip.mkv");
        fs::write(&mkv, b"x").unwrap();
        let mut side = fs::File::create(dir.path().join("clip.json")).unwrap();
        write!(side, r#"{{"title":"My Take","duration_secs":12.5}}"#).unwrap();
        let index = index_folder(dir.path());
        assert_eq!(index.entries[0].title, "My Take");
        assert!((index.entries[0].duration_secs.unwrap_or(0.0) - 12.5).abs() < 0.01);
    }
}
