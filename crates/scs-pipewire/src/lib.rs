//! PipeWire presence and device listing via `pw-dump` (no libpipewire link).

mod dump;
mod error;
mod nodes;

pub use dump::{audio_devices_from_nodes, dump_nodes, pw_record_args};
pub use error::PipewireError;
pub use nodes::{parse_pw_dump, PwNode};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PipewireStatus {
    SocketPresent { path: String },
    Missing { reason: String },
}

pub fn runtime_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
}

pub fn socket_path() -> Option<PathBuf> {
    Some(runtime_dir()?.join("pipewire-0"))
}

pub fn detect() -> PipewireStatus {
    match socket_path() {
        Some(path) if path.exists() => PipewireStatus::SocketPresent {
            path: path.display().to_string(),
        },
        Some(path) => PipewireStatus::Missing {
            reason: format!("socket not found at {}", path.display()),
        },
        None => PipewireStatus::Missing {
            reason: "XDG_RUNTIME_DIR is unset".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_does_not_panic() {
        let _ = detect();
    }
}
