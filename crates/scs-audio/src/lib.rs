//! Audio types, meters, and processing-chain models.

pub mod chain;
pub mod devices;
pub mod meter;

pub use chain::{AudioChain, AudioProcessingPreset};
pub use devices::{AudioDevice, AudioDeviceKind};
pub use meter::{analyze_i16, is_clipping, linear_to_db, meter_fraction, MeterLevels};
