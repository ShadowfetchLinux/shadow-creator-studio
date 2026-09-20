use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuSnapshot {
    pub name: String,
    pub util_percent: f32,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
    pub temp_c: f32,
}

pub fn snapshot(nvml: &Nvml) -> Option<GpuSnapshot> {
    let device = nvml.device_by_index(0).ok()?;
    let name = device.name().ok()?;
    let util = device.utilization_rates().ok()?;
    let mem = device.memory_info().ok()?;
    let temp = device.temperature(TemperatureSensor::Gpu).ok()?;
    Some(GpuSnapshot {
        name,
        util_percent: util.gpu as f32,
        vram_used_bytes: mem.used,
        vram_total_bytes: mem.total,
        temp_c: temp as f32,
    })
}

pub fn try_init() -> Option<Nvml> {
    Nvml::init().ok()
}
