use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

use scs_capture::CameraDevice;
use scs_ffmpeg::{camera_preview_rgb, desktop_preview_rgb, DesktopVideoInput};

#[derive(Debug)]
pub enum PreviewEvent {
    Frame {
        width: u32,
        height: u32,
        rgb: Vec<u8>,
    },
    Error(String),
}

pub fn start_camera(
    camera: CameraDevice,
    mirror: bool,
    stop: Arc<AtomicBool>,
) -> Receiver<PreviewEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-preview".into())
        .spawn(move || {
            let Some(fmt) = camera.preferred_format().cloned() else {
                let _ = tx.send(PreviewEvent::Error(
                    "This camera did not report a usable format.".into(),
                ));
                return;
            };
            let out_w = 640u32;
            let out_h = 360u32;
            let plan = camera_preview_rgb(
                &camera.path,
                &fmt.pixel_format,
                out_w,
                out_h,
                15,
                mirror,
            );
            let mut command = Command::new(&plan.program);
            command.args(&plan.args);
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(err) => {
                    let _ = tx.send(PreviewEvent::Error(format!(
                        "Could not start camera preview: {err}"
                    )));
                    return;
                }
            };
            let Some(mut stdout) = child.stdout.take() else {
                let _ = tx.send(PreviewEvent::Error(
                    "Camera preview produced no video stream.".into(),
                ));
                let _ = child.kill();
                return;
            };
            let frame_len = (out_w * out_h * 3) as usize;
            let mut buf = vec![0u8; frame_len];
            while !stop.load(Ordering::Relaxed) {
                if let Err(err) = stdout.read_exact(&mut buf) {
                    if !stop.load(Ordering::Relaxed) {
                        let stderr = child
                            .stderr
                            .take()
                            .and_then(|mut s| {
                                let mut t = String::new();
                                s.read_to_string(&mut t).ok();
                                Some(t)
                            })
                            .unwrap_or_default();
                        let detail = if stderr.trim().is_empty() {
                            err.to_string()
                        } else {
                            stderr
                                .lines()
                                .next()
                                .unwrap_or("preview ended")
                                .to_string()
                        };
                        let _ = tx.send(PreviewEvent::Error(human_preview_error(&detail)));
                    }
                    break;
                }
                if tx
                    .send(PreviewEvent::Frame {
                        width: out_w,
                        height: out_h,
                        rgb: buf.clone(),
                    })
                    .is_err()
                {
                    break;
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        })
        .ok();
    rx
}

pub fn start_desktop(
    input: DesktopVideoInput,
    stop: Arc<AtomicBool>,
) -> Receiver<PreviewEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-desk-preview".into())
        .spawn(move || {
            let out_w = 640u32;
            let out_h = 360u32;
            let plan = desktop_preview_rgb(&input, out_w, out_h);
            let mut command = Command::new(&plan.program);
            command.args(&plan.args);
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(err) => {
                    let _ = tx.send(PreviewEvent::Error(format!(
                        "Could not start desktop preview: {err}"
                    )));
                    return;
                }
            };
            let Some(mut stdout) = child.stdout.take() else {
                let _ = tx.send(PreviewEvent::Error(
                    "Desktop preview produced no video stream.".into(),
                ));
                let _ = child.kill();
                return;
            };
            let frame_len = (out_w * out_h * 3) as usize;
            let mut buf = vec![0u8; frame_len];
            while !stop.load(Ordering::Relaxed) {
                if let Err(err) = stdout.read_exact(&mut buf) {
                    if !stop.load(Ordering::Relaxed) {
                        let _ = tx.send(PreviewEvent::Error(format!(
                            "Desktop preview stopped: {}",
                            err
                        )));
                    }
                    break;
                }
                if tx
                    .send(PreviewEvent::Frame {
                        width: out_w,
                        height: out_h,
                        rgb: buf.clone(),
                    })
                    .is_err()
                {
                    break;
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        })
        .ok();
    rx
}

fn human_preview_error(detail: &str) -> String {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("busy") {
        "Camera is busy in another application.".into()
    } else if lower.contains("permission") {
        "Cannot open the camera — permission denied.".into()
    } else if lower.contains("no such") {
        "Camera device disappeared.".into()
    } else {
        format!("Camera preview stopped: {}", detail.chars().take(160).collect::<String>())
    }
}
