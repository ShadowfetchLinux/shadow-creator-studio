use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use uuid::Uuid;

/// Keep only ASCII letters, digits, and hyphen. Other runs become `_`.
pub fn sanitize_filename_component(raw: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            if pending_sep && !out.is_empty() {
                out.push('_');
            }
            pending_sep = false;
            out.push(ch);
        } else {
            pending_sep = true;
        }
    }
    if out.is_empty() {
        "Untitled".into()
    } else {
        out
    }
}

/// `2026-09-20_YouTube_Record_001.mkv` — increments so existing files are never overwritten.
pub fn next_recording_path(
    dir: &Path,
    date: NaiveDate,
    project: &str,
    title: &str,
    extension: &str,
) -> PathBuf {
    let project = sanitize_filename_component(project);
    let title = sanitize_filename_component(title);
    let ext = extension.trim_start_matches('.');
    let date = date.format("%Y-%m-%d");

    for seq in 1..=9999 {
        let name = format!("{date}_{project}_{title}_{seq:03}.{ext}");
        let path = dir.join(&name);
        if !path.exists() {
            return path;
        }
    }

    let name = format!(
        "{date}_{project}_{title}_9999_{}.{ext}",
        Uuid::new_v4().as_simple()
    );
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(
            sanitize_filename_component("My / Cool: Video?"),
            "My_Cool_Video"
        );
        assert_eq!(sanitize_filename_component("YouTube"), "YouTube");
        assert_eq!(sanitize_filename_component("***"), "Untitled");
    }

    #[test]
    fn first_file_is_001() {
        let dir = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let path = next_recording_path(dir.path(), date, "YouTube", "Record", "mkv");
        assert_eq!(
            path.file_name().unwrap(),
            "2026-09-20_YouTube_Record_001.mkv"
        );
    }

    #[test]
    fn never_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let first = next_recording_path(dir.path(), date, "YouTube", "Record", "mkv");
        std::fs::write(&first, b"taken").unwrap();
        let second = next_recording_path(dir.path(), date, "YouTube", "Record", "mkv");
        assert_eq!(
            second.file_name().unwrap(),
            "2026-09-20_YouTube_Record_002.mkv"
        );
        assert_ne!(first, second);
        assert!(!second.exists());
    }

    #[test]
    fn skips_holes_only_until_free_slot() {
        let dir = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        std::fs::write(
            dir.path().join("2026-01-02_Proj_Clip_001.mkv"),
            b"1",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("2026-01-02_Proj_Clip_003.mkv"),
            b"3",
        )
        .unwrap();
        let next = next_recording_path(dir.path(), date, "Proj", "Clip", "mkv");
        assert_eq!(next.file_name().unwrap(), "2026-01-02_Proj_Clip_002.mkv");
    }
}
