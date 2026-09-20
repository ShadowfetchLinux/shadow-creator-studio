//! Shared types and services that do not need GTK.
//!
//! This crate must stay free of GUI and process-spawning side effects so tests
//! and later daemons can depend on it.

pub mod disk;
pub mod error;
pub mod filenames;
pub mod markers;
pub mod migration;
pub mod modes;
pub mod paths;
pub mod quality;
pub mod recording;
pub mod redaction;
pub mod settings;
pub mod wizard;

pub use disk::{format_clock, DiskSpace};
pub use error::{CoreError, CoreResult};
pub use filenames::{next_recording_path, sanitize_filename_component};
pub use markers::{Marker, MarkerFile, MarkerKind};
pub use migration::migrate;
pub use modes::RecordingMode;
pub use quality::QualityPreset;
pub use recording::{
    stop_requires_confirmation, AudioTrackMetadata, ContainerFormat, EncoderMetadata,
    RecordingMetadata, VideoStreamMetadata,
};
pub use redaction::{redact_json, redact_text};
pub use settings::{Settings, SettingsStore, SETTINGS_VERSION};
pub use wizard::WizardState;

pub const APP_ID: &str = "com.shadowfetch.creatorstudio";
pub const APP_NAME: &str = "Shadow Creator Studio";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
