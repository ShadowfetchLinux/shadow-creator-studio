//! Host probes that are cheap enough for a 2-second UI timer.
//!
//! Never spawn `nvidia-smi` or FFmpeg from the monitor loop. GPU data uses NVML.

mod cpu;
mod disk;
mod gpu;
mod memory;
mod monitor;
mod os;
mod thermal;

pub use cpu::{parse_proc_stat_cpu_line, CpuSample};
pub use disk::probe_disk;
pub use gpu::GpuSnapshot;
pub use memory::{parse_meminfo, MemorySnapshot};
pub use monitor::{SystemMonitor, SystemSnapshot};
pub use os::os_pretty_name;
pub use thermal::cpu_temperature_c;
