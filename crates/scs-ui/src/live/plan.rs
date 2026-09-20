use chrono::Local;
use scs_audio::{AudioChain, TrackLayout};
use scs_capture::{CameraDevice, DeviceInventory};
use scs_core::filenames::next_recording_path;
use scs_core::{DiskSpace, RecordingMode};
use scs_encoder::{resolve_encoder, tune_for};
use scs_ffmpeg::{CameraInput, RecordPlanRequest};

use crate::state::StudioState;

pub fn build_request(
    state: &StudioState,
    inventory: &DeviceInventory,
    disk: Option<DiskSpace>,
) -> Result<RecordPlanRequest, String> {
    if let Some(disk) = disk {
        if disk.refuse_start() {
            return Err(
                "Not enough free disk space (need more than 1 GiB reserve). Recording did not start."
                    .into(),
            );
        }
    }
    let settings = state.settings.borrow();
    let mode = settings.last_recording_mode;
    if let Some(reason) = mode.unavailable_reason() {
        return Err(reason.into());
    }
    if !mode.records_locally() {
        return Err("This mode cannot record yet.".into());
    }
    let encoder = resolve_encoder(settings.advanced.encoder, &state.caps)?;
    let tune = tune_for(settings.recording.quality, encoder);
    let folder = settings.recording.folder.clone();
    let _ = std::fs::create_dir_all(&folder);
    let output = next_recording_path(
        &folder,
        Local::now().date_naive(),
        &settings.recording.project_title,
        &settings.recording.clip_title,
        "mkv",
    );
    let camera = match mode {
        RecordingMode::Voice => None,
        _ => Some(camera_input(
            inventory,
            settings.camera.device.as_deref(),
            settings.recording.quality.width(),
            settings.recording.quality.height(),
            settings.recording.quality.fps(),
        )?),
    };
    let mic = settings
        .audio
        .mic_device
        .clone()
        .ok_or_else(|| "Select a microphone before recording.".to_string())?;
    let desktop = match mode {
        RecordingMode::Creator | RecordingMode::Custom => settings.audio.desktop_device.clone(),
        _ => None,
    };
    let layout = TrackLayout::from_settings(&settings.audio);
    let chain = AudioChain::for_preset(settings.audio.processing_preset);
    drop(settings);
    Ok(RecordPlanRequest {
        mode,
        camera,
        mic: Some(mic),
        desktop,
        encoder,
        tune,
        output,
        layout,
        chain,
        stream_url: None,
    })
}

pub fn attach_stream(
    mut request: RecordPlanRequest,
    url: Option<String>,
    record_while_live: bool,
) -> RecordPlanRequest {
    request.stream_url = url;
    if request.stream_url.is_some() && !record_while_live {
        request.output = std::path::PathBuf::new();
    }
    request
}

fn camera_input(
    inventory: &DeviceInventory,
    selected: Option<&str>,
    max_w: u32,
    max_h: u32,
    max_fps: u32,
) -> Result<CameraInput, String> {
    let camera: &CameraDevice = selected
        .and_then(|path| inventory.cameras.iter().find(|c| c.path == path))
        .or_else(|| inventory.cameras.first())
        .ok_or_else(|| "Select a camera before recording.".to_string())?;
    let fmt = camera
        .preferred_format()
        .ok_or_else(|| "The selected camera has no usable format.".to_string())?;
    Ok(CameraInput {
        path: camera.path.clone().into(),
        pixel_format: fmt.pixel_format.clone(),
        width: fmt.width.min(max_w),
        height: fmt.height.min(max_h),
        fps: fmt.fps.unwrap_or(max_fps).min(max_fps).max(1),
    })
}
