use std::path::{Path, PathBuf};

use scs_core::RecordingMode;
use scs_encoder::{EncodeTune, VideoEncoder};

use crate::builder::{FfmpegCommandBuilder, PlannedCommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraInput {
    pub path: PathBuf,
    pub pixel_format: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPlanRequest {
    pub mode: RecordingMode,
    pub camera: Option<CameraInput>,
    pub mic: Option<String>,
    pub desktop: Option<String>,
    pub encoder: VideoEncoder,
    pub tune: EncodeTune,
    pub output: PathBuf,
}

pub fn plan_record(request: &RecordPlanRequest) -> Result<PlannedCommand, String> {
    match request.mode {
        RecordingMode::Voice => plan_voice(request),
        RecordingMode::Camera | RecordingMode::Creator | RecordingMode::Custom => {
            plan_camera(request)
        }
        RecordingMode::Screen | RecordingMode::Presentation => Err(request
            .mode
            .unavailable_reason()
            .unwrap_or("This mode cannot record yet.")
            .into()),
    }
}

fn plan_voice(request: &RecordPlanRequest) -> Result<PlannedCommand, String> {
    let mic = request
        .mic
        .as_deref()
        .ok_or_else(|| "Voice mode needs a microphone.".to_string())?;
    let mut b = FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-nostats")
        .arg("-loglevel")
        .arg("error")
        .arg("-progress")
        .arg("pipe:1")
        .never_overwrite();
    b = pulse_input(b, mic);
    b = encode_audio(b);
    Ok(b.arg("-f").arg("matroska").output(&request.output).build())
}

fn plan_camera(request: &RecordPlanRequest) -> Result<PlannedCommand, String> {
    let camera = request
        .camera
        .as_ref()
        .ok_or_else(|| "This mode needs a camera.".to_string())?;
    let mic = request
        .mic
        .as_deref()
        .ok_or_else(|| "This mode needs a microphone.".to_string())?;
    let include_desktop = matches!(request.mode, RecordingMode::Creator | RecordingMode::Custom)
        && request.desktop.is_some();

    let mut b = FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-nostats")
        .arg("-loglevel")
        .arg("error")
        .arg("-progress")
        .arg("pipe:1")
        .never_overwrite()
        .arg("-fflags")
        .arg("+genpts");
    b = v4l2_input(b, camera);
    b = pulse_input(b, mic);
    if include_desktop {
        if let Some(desktop) = request.desktop.as_deref() {
            b = pulse_input(b, desktop);
        }
    }
    b = encode_video(b, request.encoder, &request.tune, camera);
    b = encode_audio(b);
    b = b.map_stream("0:v").map_stream("1:a");
    if include_desktop {
        b = b.map_stream("2:a");
    }
    Ok(b.arg("-f").arg("matroska").output(&request.output).build())
}

fn v4l2_input(builder: FfmpegCommandBuilder, camera: &CameraInput) -> FfmpegCommandBuilder {
    let size = format!("{}x{}", camera.width, camera.height);
    builder
        .arg("-thread_queue_size")
        .arg("1024")
        .arg("-f")
        .arg("v4l2")
        .arg("-input_format")
        .arg(&camera.pixel_format)
        .arg("-video_size")
        .arg(&size)
        .arg("-framerate")
        .arg(camera.fps.max(1).to_string())
        .input(&camera.path)
}

fn pulse_input(builder: FfmpegCommandBuilder, target: &str) -> FfmpegCommandBuilder {
    builder
        .arg("-thread_queue_size")
        .arg("1024")
        .arg("-f")
        .arg("pulse")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("48000")
        .input(target)
}

fn encode_video(
    builder: FfmpegCommandBuilder,
    encoder: VideoEncoder,
    tune: &EncodeTune,
    camera: &CameraInput,
) -> FfmpegCommandBuilder {
    let bitrate = format!("{}", tune.bitrate);
    let maxrate = format!("{}", tune.maxrate);
    let size = format!("{}x{}", camera.width, camera.height);
    let mut b = builder
        .video_codec(encoder)
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-s")
        .arg(&size)
        .arg("-r")
        .arg(camera.fps.max(1).to_string());
    if matches!(encoder, VideoEncoder::Libx264) {
        b = b
            .arg("-preset")
            .arg(tune.software_preset)
            .arg("-crf")
            .arg(tune.crf.to_string());
    } else {
        b = b
            .arg("-preset")
            .arg(tune.preset)
            .arg("-rc")
            .arg("vbr")
            .arg("-cq")
            .arg(tune.cq.to_string())
            .arg("-b:v")
            .arg(&bitrate)
            .arg("-maxrate")
            .arg(&maxrate);
    }
    b
}

fn encode_audio(builder: FfmpegCommandBuilder) -> FfmpegCommandBuilder {
    builder
        .audio_codec_aac()
        .arg("-b:a")
        .arg("192k")
        .arg("-ar")
        .arg("48000")
}

pub fn sidecar_path(mkv: impl AsRef<Path>) -> PathBuf {
    mkv.as_ref().with_extension("json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scs_encoder::tune_for;
    use std::path::PathBuf;

    fn camera() -> CameraInput {
        CameraInput {
            path: PathBuf::from("/dev/video0;rm"),
            pixel_format: "mjpeg".into(),
            width: 1280,
            height: 720,
            fps: 30,
        }
    }

    fn request(mode: RecordingMode, desktop: Option<&str>) -> RecordPlanRequest {
        RecordPlanRequest {
            mode,
            camera: Some(camera()),
            mic: Some("alsa_input.usb;evil".into()),
            desktop: desktop.map(ToOwned::to_owned),
            encoder: VideoEncoder::H264Nvenc,
            tune: tune_for(scs_core::QualityPreset::Youtube1080p60, VideoEncoder::H264Nvenc),
            output: PathBuf::from("/tmp/out;rm.mkv"),
        }
    }

    fn args(cmd: &PlannedCommand) -> Vec<String> {
        cmd.args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn camera_plan_is_structured() {
        let cmd = plan_record(&request(RecordingMode::Camera, None)).unwrap();
        let args = args(&cmd);
        assert!(args.contains(&"/dev/video0;rm".into()));
        assert!(args.contains(&"alsa_input.usb;evil".into()));
        assert!(args.contains(&"h264_nvenc".into()));
        assert!(args.contains(&"-n".into()));
        assert!(args.contains(&"matroska".into()));
        assert!(!args.iter().any(|a| *a == "rm"));
        assert!(!args.iter().any(|a| a.contains("2:a")));
    }

    #[test]
    fn creator_maps_desktop_as_second_audio() {
        let cmd = plan_record(&request(
            RecordingMode::Creator,
            Some("alsa_output.speakers.monitor"),
        ))
        .unwrap();
        let args = args(&cmd);
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "2:a"));
        assert!(args.contains(&"alsa_output.speakers.monitor".into()));
    }

    #[test]
    fn screen_mode_is_refused() {
        let err = plan_record(&request(RecordingMode::Screen, None)).unwrap_err();
        assert!(err.to_ascii_lowercase().contains("desktop"));
    }

    #[test]
    fn voice_is_audio_only() {
        let mut req = request(RecordingMode::Voice, None);
        req.camera = None;
        let args = args(&plan_record(&req).unwrap());
        assert!(!args.contains(&"v4l2".into()));
        assert!(args.contains(&"aac".into()));
    }
}
