use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionId {
    Thumbnail,
    ChaptersFromMarkers,
    Scale9x16,
    SilenceDetect,
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
            id: ExtensionId::Whisper,
            label: "Transcription (Whisper)",
            available: false,
            reason: Some("Local Whisper is not bundled. Install nothing automatically."),
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
            .any(|e| e.id == ExtensionId::Whisper && !e.available));
        assert!(all
            .iter()
            .any(|e| e.id == ExtensionId::YoutubeUpload && !e.available));
        assert_eq!(available_ids().len(), 4);
    }
}
