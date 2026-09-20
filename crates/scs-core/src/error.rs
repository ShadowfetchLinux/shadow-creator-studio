use std::path::PathBuf;

use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("settings version {0} is newer than this build")]
    UnsupportedSettingsVersion(u64),

    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
}
