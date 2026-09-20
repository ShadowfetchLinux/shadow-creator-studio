use std::path::{Path, PathBuf};

/// Delete requires an explicit confirm token so UI cannot call it by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteRequest {
    pub path: PathBuf,
    pub confirmed: bool,
}

pub fn confirm_delete(request: &DeleteRequest) -> Result<PathBuf, String> {
    if !request.confirmed {
        return Err("Delete needs confirmation.".into());
    }
    if request.path.as_os_str().is_empty() {
        return Err("No file selected.".into());
    }
    Ok(request.path.clone())
}

pub fn rename_beside(path: &Path, new_stem: &str) -> Result<PathBuf, String> {
    let stem = scs_core::sanitize_filename_component(new_stem);
    if stem.is_empty() {
        return Err("The new name is empty after cleanup.".into());
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mkv");
    let dest = path.with_file_name(format!("{stem}.{ext}"));
    if dest.exists() {
        return Err("A file with that name already exists.".into());
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn delete_requires_confirm() {
        let req = DeleteRequest {
            path: PathBuf::from("/tmp/a.mkv"),
            confirmed: false,
        };
        assert!(confirm_delete(&req).is_err());
        let mut ok = req;
        ok.confirmed = true;
        assert_eq!(confirm_delete(&ok).unwrap(), PathBuf::from("/tmp/a.mkv"));
    }

    #[test]
    fn rename_sanitizes() {
        let dest = rename_beside(Path::new("/tmp/old.mkv"), "New/Name;rm").unwrap();
        assert_eq!(dest.file_name().unwrap(), "New_Name_rm.mkv");
    }
}
