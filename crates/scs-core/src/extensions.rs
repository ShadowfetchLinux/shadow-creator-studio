use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionId {
    Thumbnail,
    ChaptersFromMarkers,
    Scale9x16,
    SilenceDetect,
    SilenceRemove,
    Whisper,
    Captions,
    Highlights,
    Shorts,
    AiTitleDescription,
    YoutubeUpload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub id: ExtensionId,
    pub label: &'static str,
    pub available: bool,
    pub reason: Option<&'static str>,
}

pub fn registry() -> Vec<Extension> {
    vec![
        Extension {
            id: ExtensionId::Thumbnail,
            label: "Thumbnail",
            available: true,
            reason: None,
        },
        Extension {
            id: ExtensionId::ChaptersFromMarkers,
            label: "Chapters from markers",
            available: true,
            reason: None,
        },
        Extension {
            id: ExtensionId::Scale9x16,
            label: "9:16 scale/pad",
            available: true,
            reason: None,
        },
        Extension {
            id: ExtensionId::SilenceDetect,
            label: "Silence detect",
            available: true,
            reason: None,
        },
        Extension {
            id: ExtensionId::SilenceRemove,
            label: "Silence remove",
            available: true,
            reason: None,
        },
        Extension {
            id: ExtensionId::Whisper,
            label: "Transcription (Whisper)",
            available: whisper_ready(),
            reason: if whisper_ready() {
                None
            } else {
                Some("Needs a local whisper binary and an already-downloaded model. Nothing is downloaded automatically.")
            },
        },
        Extension {
            id: ExtensionId::Captions,
            label: "Captions",
            available: false,
            reason: Some("Needs a local transcription job first."),
        },
        Extension {
            id: ExtensionId::Highlights,
            label: "Highlights",
            available: false,
            reason: Some("No local highlight model is shipped."),
        },
        Extension {
            id: ExtensionId::Shorts,
            label: "Shorts cut",
            available: false,
            reason: Some("Use 9:16 scale/pad. Auto-Shorts is not implemented."),
        },
        Extension {
            id: ExtensionId::AiTitleDescription,
            label: "AI title / description",
            available: false,
            reason: Some("Cloud AI is not required and is not called."),
        },
        Extension {
            id: ExtensionId::YoutubeUpload,
            label: "YouTube upload",
            available: false,
            reason: Some("OAuth upload is not implemented. Use YouTube Studio."),
        },
    ]
}

pub fn whisper_ready() -> bool {
    whisper_binary().is_some() && whisper_model().is_some()
}

pub fn whisper_binary() -> Option<std::path::PathBuf> {
    let names = ["whisper-cli", "whisper-cpp", "whisper"];
    let extra = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut dirs = Vec::new();
    if let Some(home) = extra {
        dirs.push(home.join(".local/bin"));
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            dirs.push(std::path::PathBuf::from(dir));
        }
    }
    for dir in dirs {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn whisper_model() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
    let caches = [
        home.join(".cache/whisper"),
        home.join(".local/share/whisper.cpp"),
        home.join(".local/share/whisper"),
    ];
    for dir in caches {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "pt" | "bin" | "gguf") {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub fn available_ids() -> Vec<ExtensionId> {
    registry()
        .into_iter()
        .filter(|e| e.available)
        .map(|e| e.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_keeps_cloud_jobs_disabled() {
        let all = registry();
        assert!(all.iter().any(|e| e.id == ExtensionId::Thumbnail && e.available));
        assert!(all
            .iter()
            .any(|e| e.id == ExtensionId::YoutubeUpload && !e.available));
        assert!(all
            .iter()
            .any(|e| e.id == ExtensionId::AiTitleDescription && !e.available));
        if !whisper_ready() {
            assert!(all
                .iter()
                .any(|e| e.id == ExtensionId::Whisper && !e.available));
        }
        assert!(available_ids().len() >= 5);
    }
}
