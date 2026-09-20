use std::path::{Path, PathBuf};
use std::process::Command;

use crate::builder::{FfmpegCommandBuilder, PlannedCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct RemuxVerify {
    pub path: PathBuf,
    pub duration_secs: f64,
    pub format: String,
    pub size_bytes: u64,
}

pub fn remux_command(input: impl AsRef<Path>, output: impl AsRef<Path>) -> PlannedCommand {
    FfmpegCommandBuilder::remux_copy(input, output)
}

pub fn verify_media(path: impl AsRef<Path>) -> Result<RemuxVerify, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Err("The remuxed file was not created.".into());
    }
    let meta = std::fs::metadata(path).map_err(|err| err.to_string())?;
    if meta.len() == 0 {
        return Err("The remuxed file is empty. The MKV was not deleted.".into());
    }
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration,format_name,size",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(path.as_os_str())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                "ffprobe is not installed, so the MP4 cannot be verified. The MKV is kept.".into()
            } else {
                format!("Could not probe the remux: {err}")
            }
        })?;
    if !output.status.success() {
        return Err(
            "ffprobe could not read the remuxed file. The MKV is kept.".into(),
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_probe_default(&text)?;
    if parsed.duration_secs <= 0.05 {
        return Err("The remuxed file has no duration. The MKV is kept.".into());
    }
    Ok(RemuxVerify {
        path: path.to_path_buf(),
        duration_secs: parsed.duration_secs,
        format: parsed.format,
        size_bytes: if parsed.size_bytes == 0 {
            meta.len()
        } else {
            parsed.size_bytes
        },
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeFields {
    pub duration_secs: f64,
    pub format: String,
    pub size_bytes: u64,
}

pub fn parse_probe_default(text: &str) -> Result<ProbeFields, String> {
    let mut duration_secs = 0.0;
    let mut format = String::new();
    let mut size_bytes = 0;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "duration" => {
                duration_secs = value.trim().parse().map_err(|_| {
                    "ffprobe duration was not a number. The MKV is kept.".to_string()
                })?;
            }
            "format_name" => format = value.trim().to_string(),
            "size" => size_bytes = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    if format.is_empty() {
        return Err("ffprobe did not report a format. The MKV is kept.".into());
    }
    Ok(ProbeFields {
        duration_secs,
        format,
        size_bytes,
    })
}

/// Never delete the MKV. True only after a verified remux.
pub fn may_delete_mkv(mkv_exists: bool, remux_verified: bool) -> bool {
    mkv_exists && remux_verified && false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_parse_requires_duration_and_format() {
        let parsed = parse_probe_default(
            "duration=12.500000\nformat_name=mov,mp4,m4a,3gp,3g2,mj2\nsize=12345\n",
        )
        .unwrap();
        assert!((parsed.duration_secs - 12.5).abs() < 0.01);
        assert!(parsed.format.contains("mp4"));
        assert_eq!(parsed.size_bytes, 12345);
        assert!(parse_probe_default("duration=0\n").is_err());
    }

    #[test]
    fn never_authorizes_mkv_delete() {
        assert!(!may_delete_mkv(true, true));
        assert!(!may_delete_mkv(true, false));
    }

    #[test]
    fn remux_stays_copy() {
        let cmd = remux_command("/tmp/a.mkv", "/tmp/a.mp4");
        let args: Vec<_> = cmd
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(args.contains(&"-n".to_string()));
    }
}
