use std::io::Write;
use std::process::{Command, Stdio};

pub const STREAM_KEY_ATTR: &str = "youtube-stream";
pub const OBS_PASSWORD_ATTR: &str = "obs-websocket";
const SERVICE: &str = "com.shadowfetch.creatorstudio";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretLookup {
    pub present: bool,
    pub backend: SecretBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretBackend {
    SecretTool,
    Missing,
}

pub fn detect_backend() -> SecretBackend {
    Command::new("secret-tool")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .filter(|s| s.success())
        .map(|_| SecretBackend::SecretTool)
        .unwrap_or(SecretBackend::Missing)
}

pub fn lookup_stream_key() -> Result<Option<String>, String> {
    lookup_secret(STREAM_KEY_ATTR)
}

pub fn lookup_obs_password() -> Result<Option<String>, String> {
    lookup_secret(OBS_PASSWORD_ATTR)
}

fn lookup_secret(attr: &str) -> Result<Option<String>, String> {
    match detect_backend() {
        SecretBackend::Missing => Err(
            "secret-tool is not installed. Install it with: sudo apt install libsecret-tools. Secrets are never written to settings.json."
                .into(),
        ),
        SecretBackend::SecretTool => {
            let out = Command::new("secret-tool")
                .args(["lookup", "service", SERVICE, "key", attr])
                .output()
                .map_err(|e| format!("secret-tool failed: {e}"))?;
            if !out.status.success() {
                return Ok(None);
            }
            let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if key.is_empty() {
                Ok(None)
            } else {
                Ok(Some(key))
            }
        }
    }
}

pub fn store_stream_key(key: &str) -> Result<(), String> {
    store_secret(STREAM_KEY_ATTR, "Shadow Creator Studio YouTube", key)
}

pub fn store_obs_password(password: &str) -> Result<(), String> {
    store_secret(OBS_PASSWORD_ATTR, "Shadow Creator Studio OBS", password)
}

fn store_secret(attr: &str, label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("Secret is empty.".into());
    }
    if detect_backend() == SecretBackend::Missing {
        return Err(
            "secret-tool is not installed. Install it with: sudo apt install libsecret-tools. The value was not written to disk."
                .into(),
        );
    }
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            &format!("--label={label}"),
            "service",
            SERVICE,
            "key",
            attr,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start secret-tool: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(value.as_bytes())
            .map_err(|e| format!("Could not write the secret: {e}"))?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("secret-tool did not store the secret.".into())
    }
}

pub fn clear_stream_key() -> Result<(), String> {
    if detect_backend() == SecretBackend::Missing {
        return Ok(());
    }
    let _ = Command::new("secret-tool")
        .args(["clear", "service", SERVICE, "key", STREAM_KEY_ATTR])
        .status();
    Ok(())
}

pub fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "[redacted]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_key_material() {
        let raw = "rtmp://a.youtube.com/live2/abcd-live-key";
        assert_eq!(
            redact_secret(raw, "abcd-live-key"),
            "rtmp://a.youtube.com/live2/[redacted]"
        );
        assert!(!redact_secret(raw, "abcd-live-key").contains("abcd"));
    }
}
