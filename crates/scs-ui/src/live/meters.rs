use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

use scs_audio::{analyze_i16, MeterLevels};
use scs_pipewire::pw_record_args;

#[derive(Debug)]
pub enum MeterEvent {
    Levels(MeterLevels),
    Error(String),
}

pub fn start_tap(target: String, stop: Arc<AtomicBool>) -> Receiver<MeterEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-meter".into())
        .spawn(move || {
            let args = pw_record_args(&target);
            let mut command = Command::new("pw-record");
            command.args(&args);
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    let _ = tx.send(MeterEvent::Error(
                        "pw-record is not installed, so the meter cannot run.".into(),
                    ));
                    return;
                }
                Err(err) => {
                    let _ = tx.send(MeterEvent::Error(format!(
                        "Could not listen to audio: {err}"
                    )));
                    return;
                }
            };
            let Some(mut stdout) = child.stdout.take() else {
                let _ = tx.send(MeterEvent::Error("Audio tap produced no samples.".into()));
                let _ = child.kill();
                return;
            };
            let mut buf = vec![0u8; 4800 * 2];
            while !stop.load(Ordering::Relaxed) {
                if let Err(err) = stdout.read_exact(&mut buf) {
                    if !stop.load(Ordering::Relaxed) {
                        let _ = tx.send(MeterEvent::Error(human_meter_error(&err.to_string())));
                    }
                    break;
                }
                let mut samples = Vec::with_capacity(4800);
                for chunk in buf.chunks_exact(2) {
                    samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                }
                if tx.send(MeterEvent::Levels(analyze_i16(&samples))).is_err() {
                    break;
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        })
        .ok();
    rx
}

fn human_meter_error(detail: &str) -> String {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("busy") {
        "Audio device is busy.".into()
    } else if lower.contains("permission") {
        "Cannot open this audio device.".into()
    } else {
        format!("Audio meter stopped: {}", detail.chars().take(140).collect::<String>())
    }
}
