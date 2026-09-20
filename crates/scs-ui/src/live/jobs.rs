use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use scs_ffmpeg::{probe_command, PlannedCommand, ToolJob};
use scs_library::{parse_ffprobe_json, write_sidecar};

#[derive(Debug)]
#[allow(dead_code)]
pub enum JobEvent {
    Finished { message: String, output: Option<PathBuf> },
    Failed(String),
}

pub fn run_tool(job: ToolJob) -> Receiver<JobEvent> {
    spawn_cmd(job.command, Some(job.output))
}

pub fn run_tool_cmd(command: PlannedCommand) -> Receiver<JobEvent> {
    spawn_cmd(command, None)
}

fn spawn_cmd(command: PlannedCommand, output: Option<PathBuf>) -> Receiver<JobEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-tool".into())
        .spawn(move || {
            let result = Command::new(&command.program)
                .args(&command.args)
                .output();
            match result {
                Ok(out) if out.status.success() => {
                    let name = output
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Done".into());
                    let _ = tx.send(JobEvent::Finished {
                        message: format!("Wrote {name}"),
                        output,
                    });
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    let _ = tx.send(JobEvent::Failed(human_err(&err)));
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    let _ = tx.send(JobEvent::Failed(format!(
                        "{} is not installed.",
                        command.program.display()
                    )));
                }
                Err(err) => {
                    let _ = tx.send(JobEvent::Failed(format!("Could not start the tool: {err}")));
                }
            }
        })
        .ok();
    rx
}

pub fn probe_and_write_sidecar(path: PathBuf) -> Receiver<JobEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-probe".into())
        .spawn(move || {
            let cmd = probe_command(&path);
            match Command::new(&cmd.program).args(&cmd.args).output() {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    match parse_ffprobe_json(&text) {
                        Ok(mut side) => {
                            side.title = path.file_stem().map(|s| s.to_string_lossy().into_owned());
                            match write_sidecar(&path, &side) {
                                Ok(_) => {
                                    let _ = tx.send(JobEvent::Finished {
                                        message: "Updated sidecar metadata".into(),
                                        output: Some(path),
                                    });
                                }
                                Err(err) => {
                                    let _ = tx.send(JobEvent::Failed(err));
                                }
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(JobEvent::Failed(err));
                        }
                    }
                }
                Ok(_) => {
                    let _ = tx.send(JobEvent::Failed(
                        "ffprobe could not read that file.".into(),
                    ));
                }
                Err(_) => {
                    let _ = tx.send(JobEvent::Failed(
                        "ffprobe is not installed, so metadata stays unknown.".into(),
                    ));
                }
            }
        })
        .ok();
    rx
}

fn human_err(stderr: &str) -> String {
    let line = stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("The FFmpeg tool failed.");
    if line.len() > 240 {
        format!("{}…", &line[..240])
    } else {
        line.to_string()
    }
}

pub fn open_path(path: &std::path::Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path.as_os_str())
        .spawn()
        .map(|_| ())
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                "xdg-open is not installed.".into()
            } else {
                format!("Could not open: {err}")
            }
        })
}
