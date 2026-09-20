use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use scs_ffmpeg::{
    human_ffmpeg_error, parse_progress_block, plan_desktop_record, plan_record, remux_command,
    verify_media, DesktopVideoInput, GstPip, RecordPlanRequest,
};
use scs_library::{write_sidecar, Sidecar};
use scs_obs::{ObsConnectionConfig, ObsSession};

#[derive(Debug)]
pub enum RecordEvent {
    Started { path: PathBuf, encoder: String },
    Progress { frame: u64, out_time_ms: u64, drop_frames: Option<u64> },
    Warning(String),
    Finished { message: String, remuxed: Option<PathBuf> },
    Failed(String),
}

pub fn start_desktop_session(
    input: DesktopVideoInput,
    mic: Option<String>,
    pip: Option<GstPip>,
    output: PathBuf,
    prefer_nvenc: bool,
    rtmp_url: Option<String>,
    remux: bool,
    stop: Arc<AtomicBool>,
) -> Receiver<RecordEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-gst-record".into())
        .spawn(move || {
            if let Err(err) = run_desktop_session(
                input, mic, pip, output, prefer_nvenc, rtmp_url, remux, stop, &tx,
            ) {
                let _ = tx.send(RecordEvent::Failed(err));
            }
        })
        .ok();
    rx
}

pub fn start_obs_session(stream: bool, stop: Arc<AtomicBool>) -> Receiver<RecordEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-obs-record".into())
        .spawn(move || {
            if let Err(err) = run_obs_session(stream, stop, &tx) {
                let _ = tx.send(RecordEvent::Failed(err));
            }
        })
        .ok();
    rx
}

pub fn start_session(
    request: RecordPlanRequest,
    remux: bool,
    stop: Arc<AtomicBool>,
) -> Receiver<RecordEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-record".into())
        .spawn(move || {
            if let Err(err) = run_session(request, remux, stop, &tx) {
                let _ = tx.send(RecordEvent::Failed(err));
            }
        })
        .ok();
    rx
}

fn run_session(
    request: RecordPlanRequest,
    remux: bool,
    stop: Arc<AtomicBool>,
    tx: &std::sync::mpsc::Sender<RecordEvent>,
) -> Result<(), String> {
    let encoder = request.encoder.ffmpeg_name().to_string();
    let mkv = request.output.clone();
    let plan = plan_record(&request)?;
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            "ffmpeg is not installed, so recording cannot start.".into()
        } else {
            format!("Could not start ffmpeg: {err}")
        }
    })?;
    let mut stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let _ = tx.send(RecordEvent::Started {
        path: mkv.clone(),
        encoder,
    });

    let err_buf = Arc::new(std::sync::Mutex::new(String::new()));
    if let Some(stderr) = stderr {
        let err_buf = Arc::clone(&err_buf);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(mut buf) = err_buf.lock() {
                    if buf.len() < 4000 {
                        buf.push_str(&line);
                    }
                }
                line.clear();
            }
        });
    }

    if let Some(mut stdout) = stdout {
        let mut leftover = String::new();
        let mut bytes = [0u8; 512];
        loop {
            if stop.load(Ordering::Relaxed) {
                if let Some(stdin) = stdin.as_mut() {
                    let _ = stdin.write_all(b"q");
                    let _ = stdin.flush();
                }
                break;
            }
            match stdout.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => {
                    leftover.push_str(&String::from_utf8_lossy(&bytes[..n]));
                    while let Some(idx) = leftover.find("progress=") {
                        let end = leftover[idx..]
                            .find('\n')
                            .map(|i| idx + i + 1)
                            .unwrap_or(leftover.len());
                        let block = leftover[..end].to_string();
                        leftover = leftover[end..].to_string();
                        let parsed = parse_progress_block(&block);
                        let _ = tx.send(RecordEvent::Progress {
                            frame: parsed.frame,
                            out_time_ms: parsed.out_time_ms,
                            drop_frames: parsed.drop_frames,
                        });
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    }

    if stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(400));
        let _ = child.try_wait();
    }
    match child.try_wait() {
        Ok(Some(_)) => {}
        _ => {
            if stop.load(Ordering::Relaxed) {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }

    let stderr_text = err_buf.lock().map(|s| s.clone()).unwrap_or_default();
    if !mkv.exists() {
        return Err(if stderr_text.trim().is_empty() {
            "FFmpeg exited before a file was written.".into()
        } else {
            human_ffmpeg_error(&stderr_text)
        });
    }

    let mut message = format!("Saved {}", file_name(&mkv));
    let mut remuxed = None;
    if remux {
        match remux_take(&mkv) {
            Ok(path) => {
                remuxed = Some(path.clone());
                message = format!(
                    "Saved {} and a verified MP4 copy. The MKV was kept.",
                    file_name(&mkv)
                );
            }
            Err(err) => {
                let _ = tx.send(RecordEvent::Warning(err));
            }
        }
    }
    write_take_sidecar(&mkv, &request);
    let _ = tx.send(RecordEvent::Finished { message, remuxed });
    Ok(())
}

fn run_desktop_session(
    input: DesktopVideoInput,
    mic: Option<String>,
    pip: Option<GstPip>,
    output: PathBuf,
    prefer_nvenc: bool,
    rtmp_url: Option<String>,
    remux: bool,
    stop: Arc<AtomicBool>,
    tx: &std::sync::mpsc::Sender<RecordEvent>,
) -> Result<(), String> {
    if !scs_ffmpeg::gst_available() {
        return Err("gst-launch-1.0 is not installed, so desktop recording cannot start.".into());
    }
    let plan = plan_desktop_record(
        &input,
        mic.as_deref(),
        pip.as_ref(),
        &output,
        prefer_nvenc,
        rtmp_url.as_deref(),
    );
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            "gst-launch-1.0 is not installed, so desktop recording cannot start.".into()
        } else {
            format!("Could not start GStreamer: {err}")
        }
    })?;
    let encoder = if prefer_nvenc {
        "nvh264enc"
    } else {
        "x264enc"
    };
    let _ = tx.send(RecordEvent::Started {
        path: output.clone(),
        encoder: encoder.into(),
    });
    let err_buf = Arc::new(std::sync::Mutex::new(String::new()));
    if let Some(stderr) = child.stderr.take() {
        let err_buf = Arc::clone(&err_buf);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(mut buf) = err_buf.lock() {
                    if buf.len() < 4000 {
                        buf.push_str(&line);
                    }
                }
                line.clear();
            }
        });
    }
    let mut frames = 0u64;
    while !stop.load(Ordering::Relaxed) {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                frames += 1;
                let _ = tx.send(RecordEvent::Progress {
                    frame: frames,
                    out_time_ms: frames * 250,
                    drop_frames: None,
                });
                thread::sleep(Duration::from_millis(250));
            }
            Err(_) => break,
        }
    }
    if stop.load(Ordering::Relaxed) {
        let _ = child.kill();
    }
    let _ = child.wait();
    if !output.exists() {
        let stderr_text = err_buf.lock().map(|s| s.clone()).unwrap_or_default();
        return Err(if stderr_text.trim().is_empty() {
            "GStreamer exited before a file was written.".into()
        } else {
            format!(
                "Desktop record failed: {}",
                stderr_text.lines().next().unwrap_or("gstreamer error")
            )
        });
    }
    let mut message = format!("Saved {}", file_name(&output));
    let mut remuxed = None;
    if remux {
        match remux_take(&output) {
            Ok(path) => {
                remuxed = Some(path);
                message = format!(
                    "Saved {} and a verified MP4 copy. The MKV was kept.",
                    file_name(&output)
                );
            }
            Err(err) => {
                let _ = tx.send(RecordEvent::Warning(err));
            }
        }
    }
    let _ = tx.send(RecordEvent::Finished { message, remuxed });
    Ok(())
}

fn run_obs_session(
    stream: bool,
    stop: Arc<AtomicBool>,
    tx: &std::sync::mpsc::Sender<RecordEvent>,
) -> Result<(), String> {
    let password = match scs_core::lookup_obs_password() {
        Ok(pw) => pw,
        Err(err) => {
            if err.contains("secret-tool") {
                None
            } else {
                return Err(err);
            }
        }
    };
    let mut session = ObsSession::connect(&ObsConnectionConfig::default(), password.as_deref())?;
    if stream {
        session.start_stream()?;
    } else {
        session.start_record()?;
    }
    let _ = tx.send(RecordEvent::Started {
        path: PathBuf::from("OBS output folder"),
        encoder: "OBS WebSocket".into(),
    });
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(250));
    }
    let stop_result = if stream {
        session.stop_stream()
    } else {
        session.stop_record()
    };
    if let Err(err) = stop_result {
        let _ = tx.send(RecordEvent::Warning(err));
    }
    let _ = tx.send(RecordEvent::Finished {
        message: if stream {
            "OBS stream stopped.".into()
        } else {
            "OBS recording stopped. The file is in the OBS recordings folder.".into()
        },
        remuxed: None,
    });
    Ok(())
}

fn write_take_sidecar(mkv: &PathBuf, request: &RecordPlanRequest) {
    let mut side = Sidecar {
        title: mkv.file_stem().map(|s| s.to_string_lossy().into_owned()),
        video_codec: Some(request.encoder.ffmpeg_name().into()),
        audio_codec: Some("aac".into()),
        ..Sidecar::default()
    };
    if let Some(cam) = &request.camera {
        side.width = Some(cam.width);
        side.height = Some(cam.height);
        side.fps = Some(cam.fps as f32);
    }
    let _ = write_sidecar(mkv, &side);
}

fn remux_take(mkv: &PathBuf) -> Result<PathBuf, String> {
    let mp4 = mkv.with_extension("mp4");
    if mp4.exists() {
        return Err("An MP4 with that name already exists. The MKV was not replaced.".into());
    }
    let plan = remux_command(mkv, &mp4);
    let output = Command::new(&plan.program)
        .args(&plan.args)
        .output()
        .map_err(|err| format!("Remux failed to start: {err}"))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&mp4);
        return Err("Copy remux failed. The MKV is the recording.".into());
    }
    verify_media(&mp4)?;
    Ok(mp4)
}

fn file_name(path: &PathBuf) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
