//! Audio types. Capture and processing start in M2 / M4.

pub mod chain;
pub mod devices;

pub use chain::{AudioChain, AudioProcessingPreset};
pub use devices::{AudioDevice, AudioDeviceKind};
