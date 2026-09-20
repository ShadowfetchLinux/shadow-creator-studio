use std::path::{Path, PathBuf};

use nvml_wrapper::Nvml;
use scs_core::DiskSpace;
use thiserror::Error;

use crate::cpu::{read_cpu_sample, CpuSample};
use crate::disk::probe_disk;
use crate::gpu::{self, GpuSnapshot};
use crate::memory::read_memory;
use crate::thermal::cpu_temperature_c;

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("disk probe failed: {0}")]
    Disk(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemSnapshot {
    pub cpu_percent: Option<f32>,
    pub ram_used_bytes: Option<u64>,
    pub ram_total_bytes: Option<u64>,
    pub gpu_name: Option<String>,
    pub gpu_util_percent: Option<f32>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub gpu_temp_c: Option<f32>,
    pub cpu_temp_c: Option<f32>,
    pub disk: Option<DiskSpace>,
    pub bitrate_kbps: Option<f32>,
    pub dropped_frames: Option<u64>,
    pub encode_fps: Option<f32>,
    pub disk_write_bps: Option<u64>,
}

impl SystemSnapshot {
    pub fn unavailable() -> Self {
        Self {
            cpu_percent: None,
            ram_used_bytes: None,
            ram_total_bytes: None,
            gpu_name: None,
            gpu_util_percent: None,
            vram_used_bytes: None,
            vram_total_bytes: None,
            gpu_temp_c: None,
            cpu_temp_c: None,
            disk: None,
            bitrate_kbps: None,
            dropped_frames: None,
            encode_fps: None,
            disk_write_bps: None,
        }
    }
}

pub struct SystemMonitor {
    prev_cpu: Option<CpuSample>,
    nvml: Option<Nvml>,
    disk_path: PathBuf,
}

impl SystemMonitor {
    pub fn new(disk_path: impl Into<PathBuf>) -> Self {
        Self {
            prev_cpu: read_cpu_sample(),
            nvml: gpu::try_init(),
            disk_path: disk_path.into(),
        }
    }

    pub fn set_disk_path(&mut self, path: impl Into<PathBuf>) {
        self.disk_path = path.into();
    }

    pub fn snapshot(&mut self) -> SystemSnapshot {
        let mut snap = SystemSnapshot::unavailable();

        if let Some(sample) = read_cpu_sample() {
            if let Some(prev) = self.prev_cpu {
                snap.cpu_percent = prev.percent_since(sample);
            }
            self.prev_cpu = Some(sample);
        }

        if let Some(mem) = read_memory() {
            snap.ram_used_bytes = Some(mem.used_bytes());
            snap.ram_total_bytes = Some(mem.total_bytes);
        }

        snap.cpu_temp_c = cpu_temperature_c();

        if let Some(nvml) = self.nvml.as_ref() {
            if let Some(gpu) = gpu::snapshot(nvml) {
                apply_gpu(&mut snap, gpu);
            }
        }

        snap.disk = probe_disk(disk_or_home(&self.disk_path)).ok();
        snap
    }
}

fn apply_gpu(snap: &mut SystemSnapshot, gpu: GpuSnapshot) {
    snap.gpu_name = Some(gpu.name);
    snap.gpu_util_percent = Some(gpu.util_percent);
    snap.vram_used_bytes = Some(gpu.vram_used_bytes);
    snap.vram_total_bytes = Some(gpu.vram_total_bytes);
    snap.gpu_temp_c = Some(gpu.temp_c);
}

fn disk_or_home(path: &Path) -> &Path {
    if path.exists() {
        path
    } else {
        Path::new("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_fills_proc_metrics() {
        let mut monitor = SystemMonitor::new("/");
        let first = monitor.snapshot();
        let second = monitor.snapshot();
        assert!(second.ram_total_bytes.unwrap_or(0) > 0);
        assert!(second.disk.is_some());
        // First sample may lack a delta; second should usually have CPU.
        let _ = first.cpu_percent;
    }
}
