use std::process::Command;

use chrono::{DateTime, Utc};
use scs_core::{redact_json, redact_text, Settings, APP_NAME, APP_VERSION};
use scs_encoder::{capabilities_from_encoder_list, EncoderCapabilities};
use scs_obs::{detect_install, ObsInstallStatus};
use scs_pipewire::{detect as detect_pipewire, PipewireStatus};
use scs_system::{os_pretty_name, SystemSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Probe {
    Available { value: String },
    Unavailable { reason: String },
}

impl Probe {
    pub fn available(value: impl Into<String>) -> Self {
        Self::Available { value: value.into() }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Available { value } => value.clone(),
            Self::Unavailable { reason } => format!("Unavailable — {reason}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub generated_at: DateTime<Utc>,
    pub app: String,
    pub app_version: String,
    pub os: Probe,
    pub rustc: Probe,
    pub ffmpeg: Probe,
    pub pipewire: Probe,
    pub pipewire_socket: Probe,
    pub gpu: Probe,
    pub nvml: Probe,
    pub obs: Probe,
    pub easyeffects: Probe,
    pub encoder_capabilities: EncoderCapabilities,
    pub settings_redacted: Value,
}

impl DiagnosticReport {
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{APP_NAME} — Diagnostic Report\nGenerated: {}\nRedaction: secrets stripped\n\n",
            self.generated_at.to_rfc3339()
        ));
        out.push_str(&format!("App: {} {}\n", self.app, self.app_version));
        out.push_str(&format!("OS: {}\n", self.os.display()));
        out.push_str(&format!("Rustc: {}\n", self.rustc.display()));
        out.push_str(&format!("FFmpeg: {}\n", self.ffmpeg.display()));
        out.push_str(&format!("PipeWire: {}\n", self.pipewire.display()));
        out.push_str(&format!(
            "PipeWire socket: {}\n",
            self.pipewire_socket.display()
        ));
        out.push_str(&format!("GPU: {}\n", self.gpu.display()));
        out.push_str(&format!("NVML: {}\n", self.nvml.display()));
        out.push_str(&format!("OBS: {}\n", self.obs.display()));
        out.push_str(&format!("EasyEffects: {}\n", self.easyeffects.display()));
        out.push_str("\n== Encoder capabilities ==\n");
        out.push_str(&format!(
            "h264_nvenc: {}\nhevc_nvenc: {}\nav1_nvenc: {}\nlibx264: {}\nlibx265: {}\n",
            cap(&self.encoder_capabilities.h264_nvenc),
            cap(&self.encoder_capabilities.hevc_nvenc),
            cap(&self.encoder_capabilities.av1_nvenc),
            cap(&self.encoder_capabilities.libx264),
            cap(&self.encoder_capabilities.libx265),
        ));
        out.push_str("\n== Settings (redacted) ==\n");
        out.push_str(&serde_json::to_string_pretty(&self.settings_redacted).unwrap_or_default());
        out.push('\n');
        redact_text(&out)
    }
}

fn cap(c: &scs_encoder::Capability) -> String {
    match c {
        scs_encoder::Capability::Available { notes } => format!("Available — {notes}"),
        scs_encoder::Capability::Unavailable { reason } => format!("Unavailable — {reason}"),
        scs_encoder::Capability::Unknown => "Unavailable — not probed".into(),
    }
}

pub fn collect_report(settings: &Settings, snapshot: Option<&SystemSnapshot>) -> DiagnosticReport {
    let ffmpeg = first_line("ffmpeg", &["-version"]);
    let encoder_capabilities = match run_stdout("ffmpeg", &["-hide_banner", "-encoders"]) {
        Some(text) => capabilities_from_encoder_list(&text),
        None => EncoderCapabilities::unknown(),
    };

    let mut settings_redacted = serde_json::to_value(settings).unwrap_or(Value::Null);
    redact_json(&mut settings_redacted);

    let gpu = match snapshot.and_then(|s| s.gpu_name.clone()) {
        Some(name) => {
            let extra = snapshot
                .and_then(|s| s.vram_total_bytes)
                .map(|bytes| format!("{name} ({:.1} GiB VRAM)", bytes as f64 / 1024.0 / 1024.0 / 1024.0))
                .unwrap_or(name);
            Probe::available(extra)
        }
        None => Probe::unavailable("NVML did not return a GPU name"),
    };

    let nvml = match snapshot.and_then(|s| s.gpu_name.as_ref()) {
        Some(_) => Probe::available("initialized"),
        None => Probe::unavailable("NVML not initialized"),
    };

    DiagnosticReport {
        generated_at: Utc::now(),
        app: APP_NAME.into(),
        app_version: APP_VERSION.into(),
        os: os_pretty_name()
            .map(Probe::available)
            .unwrap_or_else(|| Probe::unavailable("could not read /etc/os-release")),
        rustc: first_line("rustc", &["--version"]),
        ffmpeg,
        pipewire: first_line("pipewire", &["--version"]),
        pipewire_socket: match detect_pipewire() {
            PipewireStatus::SocketPresent { path } => Probe::available(path),
            PipewireStatus::Missing { reason } => Probe::unavailable(reason),
        },
        gpu,
        nvml,
        obs: match detect_install() {
            ObsInstallStatus::Found { path } => Probe::available(path),
            ObsInstallStatus::Missing => Probe::unavailable("obs binary not found"),
        },
        easyeffects: match which("easyeffects") {
            Some(path) => Probe::available(path),
            None => Probe::unavailable("not on PATH"),
        },
        encoder_capabilities,
        settings_redacted,
    }
}

fn first_line(program: &str, args: &[&str]) -> Probe {
    match run_stdout(program, args) {
        Some(text) => {
            let line = text.lines().next().unwrap_or("").trim();
            if line.is_empty() {
                Probe::unavailable("empty output")
            } else {
                Probe::available(line)
            }
        }
        None => Probe::unavailable(format!("{program} not available")),
    }
}

fn run_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn which(program: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use scs_core::Settings;

    #[test]
    fn report_redacts_stream_key_ref() {
        let mut settings = Settings::recommended();
        settings.streaming.stream_key_ref = Some("live_should_never_appear".into());
        let report = collect_report(&settings, None);
        let text = report.render_text();
        assert!(!text.contains("live_should_never_appear"), "{text}");
        assert!(text.contains("[redacted]") || text.contains("stream_key_ref"));
        let dumped = serde_json::to_string(&report.settings_redacted).unwrap();
        assert!(!dumped.contains("live_should_never_appear"));
    }

    #[test]
    fn probe_display_is_honest() {
        assert_eq!(
            Probe::unavailable("not probed").display(),
            "Unavailable — not probed"
        );
        assert_eq!(Probe::available("1.80.0").display(), "1.80.0");
    }
}
