use std::fmt;

/// Failures while talking to PipeWire tools. Never includes secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipewireError {
    NotRunning,
    CommandMissing { program: String },
    Failed { program: String, detail: String },
    Parse(String),
}

impl fmt::Display for PipewireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.human_message())
    }
}

impl std::error::Error for PipewireError {}

impl PipewireError {
    pub fn human_message(&self) -> String {
        match self {
            Self::NotRunning => {
                "PipeWire is not running. Start the session and try again.".into()
            }
            Self::CommandMissing { program } => {
                format!("{program} is not installed, so devices cannot be listed.")
            }
            Self::Failed { program, detail } => {
                format!("{program} failed: {}", sanitize_detail(detail))
            }
            Self::Parse(detail) => format!("Could not read PipeWire device list ({detail})."),
        }
    }
}

pub fn map_spawn_error(program: &str, err: &std::io::Error) -> PipewireError {
    if err.kind() == std::io::ErrorKind::NotFound {
        PipewireError::CommandMissing {
            program: program.into(),
        }
    } else {
        PipewireError::Failed {
            program: program.into(),
            detail: err.to_string(),
        }
    }
}

pub fn map_command_failure(program: &str, stderr: &str, status: Option<i32>) -> PipewireError {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("connection refused") || lower.contains("no such file") && lower.contains("pipewire")
    {
        return PipewireError::NotRunning;
    }
    if lower.contains("permission denied") {
        return PipewireError::Failed {
            program: program.into(),
            detail: "permission denied — the session may block this device".into(),
        };
    }
    if lower.contains("busy") || lower.contains("device or resource busy") {
        return PipewireError::Failed {
            program: program.into(),
            detail: "device is busy in another application".into(),
        };
    }
    let snippet = stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("unknown error");
    PipewireError::Failed {
        program: program.into(),
        detail: match status {
            Some(code) => format!("exit {code}: {snippet}"),
            None => snippet.into(),
        },
    }
}

fn sanitize_detail(detail: &str) -> String {
    detail
        .replace("/home/", "~/")
        .chars()
        .take(240)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_failures_to_human_text() {
        assert!(map_command_failure("pw-dump", "Connection refused", Some(1))
            .human_message()
            .contains("not running"));
        assert!(map_command_failure("pw-record", "Device or resource busy", Some(1))
            .human_message()
            .contains("busy"));
        assert!(map_command_failure("pw-record", "Permission denied opening node", Some(1))
            .human_message()
            .to_ascii_lowercase()
            .contains("permission"));
        let missing = map_spawn_error(
            "pw-dump",
            &std::io::Error::new(std::io::ErrorKind::NotFound, "x"),
        );
        assert!(matches!(missing, PipewireError::CommandMissing { .. }));
    }
}
