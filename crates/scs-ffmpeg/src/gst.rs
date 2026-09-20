use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::builder::PlannedCommand;

/// GStreamer desktop capture. `fd` is inherited by the child (clear CLOEXEC first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopVideoInput {
    pub node_id: u32,
    pub fd: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GstPip {
    pub camera_path: PathBuf,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn gst_available() -> bool {
    std::process::Command::new("gst-launch-1.0")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn has_element(name: &str) -> bool {
    std::process::Command::new("gst-inspect-1.0")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn video_encoder_args(prefer_nvenc: bool) -> Vec<OsString> {
    if prefer_nvenc {
        vec![
            "nvh264enc".into(),
            "preset=low-latency-hq".into(),
            "bitrate=6000".into(),
            "!".into(),
            "h264parse".into(),
            "!".into(),
        ]
    } else {
        vec![
            "x264enc".into(),
            "tune=zerolatency".into(),
            "speed-preset=veryfast".into(),
            "!".into(),
            "h264parse".into(),
            "!".into(),
        ]
    }
}

fn pw_src(fd: i32, node_id: u32) -> Vec<OsString> {
    vec![
        format!("pipewiresrc").into(),
        format!("fd={fd}").into(),
        format!("target-object={node_id}").into(),
        "do-timestamp=true".into(),
        "!".into(),
        "videoconvert".into(),
        "!".into(),
    ]
}

/// RGB24 frames on stdout for the GTK preview.
pub fn desktop_preview_rgb(input: &DesktopVideoInput, width: u32, height: u32) -> PlannedCommand {
    let mut args: Vec<OsString> = vec!["-q".into()];
    args.extend(pw_src(input.fd, input.node_id));
    args.extend([
        "videoscale".into(),
        "!".into(),
        format!("video/x-raw,format=RGB,width={width},height={height}").into(),
        "!".into(),
        "fdsink".into(),
        "fd=1".into(),
        "sync=false".into(),
    ]);
    PlannedCommand {
        program: PathBuf::from("gst-launch-1.0"),
        args,
    }
}

pub fn plan_desktop_record(
    input: &DesktopVideoInput,
    mic: Option<&str>,
    pip: Option<&GstPip>,
    output: &Path,
    prefer_nvenc: bool,
    rtmp_url: Option<&str>,
) -> PlannedCommand {
    let mut args: Vec<OsString> = vec!["-e".into(), "-q".into()];
    let canvas_w = input.width.max(640);
    let canvas_h = input.height.max(360);

    if let Some(pip) = pip {
        args.extend([
            "compositor".into(),
            "name=comp".into(),
            "background=0".into(),
            format!("sink_1::xpos={}", pip.x).into(),
            format!("sink_1::ypos={}", pip.y).into(),
            format!("sink_1::width={}", pip.width).into(),
            format!("sink_1::height={}", pip.height).into(),
            "!".into(),
            "videoconvert".into(),
            "!".into(),
        ]);
        args.extend(video_encoder_args(prefer_nvenc));
        args.extend(["mux.".into()]);
        args.extend(pw_src(input.fd, input.node_id));
        args.extend([
            "videoscale".into(),
            "!".into(),
            format!("video/x-raw,width={canvas_w},height={canvas_h}").into(),
            "!".into(),
            "comp.sink_0".into(),
        ]);
        args.extend([
            "v4l2src".into(),
            format!("device={}", pip.camera_path.display()).into(),
            "do-timestamp=true".into(),
            "!".into(),
            "videoconvert".into(),
            "!".into(),
            "videoscale".into(),
            "!".into(),
            format!("video/x-raw,width={},height={}", pip.width, pip.height).into(),
            "!".into(),
            "comp.sink_1".into(),
        ]);
    } else {
        args.extend(pw_src(input.fd, input.node_id));
        args.extend(["videoscale".into(), "!".into()]);
        args.extend(video_encoder_args(prefer_nvenc));
        args.extend(["mux.".into()]);
    }

    if let Some(mic) = mic {
        args.extend([
            "pulsesrc".into(),
            format!("device={mic}").into(),
            "!".into(),
            "audioconvert".into(),
            "!".into(),
            "audioresample".into(),
            "!".into(),
            "avenc_aac".into(),
            "!".into(),
            "mux.".into(),
        ]);
    }

    if let Some(url) = rtmp_url {
        args.extend([
            "tee".into(),
            "name=t".into(),
            "!".into(),
            "queue".into(),
            "!".into(),
            "matroskamux".into(),
            "name=mux".into(),
            "!".into(),
            "filesink".into(),
            format!("location={}", output.display()).into(),
            "t.".into(),
            "!".into(),
            "queue".into(),
            "!".into(),
            "flvmux".into(),
            "streamable=true".into(),
            "!".into(),
            "rtmpsink".into(),
            format!("location={url}").into(),
        ]);
    } else {
        args.extend([
            "matroskamux".into(),
            "name=mux".into(),
            "!".into(),
            "filesink".into(),
            format!("location={}", output.display()).into(),
        ]);
    }

    PlannedCommand {
        program: PathBuf::from("gst-launch-1.0"),
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(cmd: &PlannedCommand) -> Vec<String> {
        cmd.args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn preview_uses_numeric_node_and_fd() {
        let input = DesktopVideoInput {
            node_id: 77,
            fd: 12,
            width: 1920,
            height: 1080,
        };
        let a = args(&desktop_preview_rgb(&input, 640, 360));
        assert!(a.iter().any(|x| x == "fd=12"));
        assert!(a.iter().any(|x| x == "target-object=77"));
        assert!(a.iter().any(|x| x.contains("RGB")));
        assert!(!a.iter().any(|x| x.contains(';')));
    }

    #[test]
    fn record_keeps_mic_as_single_argv() {
        let input = DesktopVideoInput {
            node_id: 3,
            fd: 9,
            width: 1920,
            height: 1080,
        };
        let nasty = "alsa_input.usb;evil";
        let cmd = plan_desktop_record(
            &input,
            Some(nasty),
            None,
            Path::new("/tmp/out;rm.mkv"),
            true,
            None,
        );
        let a = args(&cmd);
        assert!(a.contains(&format!("device={nasty}")));
        assert!(a.contains(&"location=/tmp/out;rm.mkv".to_string()));
        assert!(a.contains(&"nvh264enc".to_string()));
        assert!(!a.iter().any(|x| *x == "rm"));
    }

    #[test]
    fn presentation_includes_compositor_and_v4l2() {
        let input = DesktopVideoInput {
            node_id: 5,
            fd: 8,
            width: 1920,
            height: 1080,
        };
        let pip = GstPip {
            camera_path: PathBuf::from("/dev/video0;rm"),
            x: 1416,
            y: 786,
            width: 480,
            height: 270,
        };
        let a = args(&plan_desktop_record(
            &input,
            Some("mic"),
            Some(&pip),
            Path::new("/tmp/p.mkv"),
            false,
            None,
        ));
        assert!(a.contains(&"compositor".to_string()));
        assert!(a.iter().any(|x| x.contains("video0;rm")));
        assert!(a.contains(&"x264enc".to_string()));
    }
}
