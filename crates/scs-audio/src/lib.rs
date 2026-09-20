//! Audio types, meters, and processing-chain models.

pub mod calibrate;
pub mod chain;
pub mod devices;
pub mod filters;
pub mod meter;
pub mod monitor;
pub mod tracks;

pub use calibrate::{recommend_from_levels, recommend_from_peak, CalibrationAdvice};
pub use chain::{AudioChain, AudioProcessingPreset};
pub use devices::{AudioDevice, AudioDeviceKind};
pub use filters::{build_filter_graph, FilterGraph};
pub use meter::{analyze_i16, is_clipping, linear_to_db, meter_fraction, MeterLevels};
pub use monitor::{looks_like_headphone, monitor_policy, MonitorPolicy};
pub use tracks::{AudioBus, TrackLayout, TrackRole};
