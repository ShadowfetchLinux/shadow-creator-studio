use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 4455;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObsConnectionConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ObsConnectionConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.into(),
            port: DEFAULT_PORT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObsInstallStatus {
    Found { path: String },
    Missing,
}

pub fn candidate_binaries() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(".local/bin/obs"));
    }
    paths.push(PathBuf::from("/usr/bin/obs"));
    paths.push(PathBuf::from("/usr/local/bin/obs"));
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = Path::new(dir).join("obs");
            if !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }
    }
    paths
}

pub fn detect_install() -> ObsInstallStatus {
    for path in candidate_binaries() {
        if path.is_file() {
            return ObsInstallStatus::Found {
                path: path.display().to_string(),
            };
        }
    }
    ObsInstallStatus::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_websocket_endpoint() {
        let cfg = ObsConnectionConfig::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4455);
    }

    #[test]
    fn detect_does_not_panic() {
        let _ = detect_install();
    }
}
