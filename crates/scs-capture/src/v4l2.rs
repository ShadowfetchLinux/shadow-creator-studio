use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{map_tool_text, CaptureError};
use crate::formats::parse_v4l2_list_formats;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraFormat {
    pub pixel_format: String,
    pub width: u32,
    pub height: u32,
    pub fps: Option<u32>,
}

impl CameraFormat {
    pub fn label(&self) -> String {
        match self.fps {
            Some(fps) => format!(
                "{}×{} @ {} · {}",
                self.width,
                self.height,
                fps,
                self.pixel_format.to_ascii_uppercase()
            ),
            None => format!(
                "{}×{} · {}",
                self.width,
                self.height,
                self.pixel_format.to_ascii_uppercase()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDevice {
    pub path: String,
    pub name: String,
    pub formats: Vec<CameraFormat>,
}

impl CameraDevice {
    pub fn preferred_format(&self) -> Option<&CameraFormat> {
        prefer_format(&self.formats)
    }

    pub fn label(&self) -> String {
        match self.preferred_format() {
            Some(fmt) => format!("{} · {}", self.name, fmt.label()),
            None => self.name.clone(),
        }
    }
}

pub fn prefer_format(formats: &[CameraFormat]) -> Option<&CameraFormat> {
    let ranked = |fmt: &CameraFormat| {
        let pix = if fmt.pixel_format == "mjpeg" {
            0
        } else if fmt.pixel_format == "yuyv422" {
            1
        } else {
            2
        };
        let area = fmt.width.saturating_mul(fmt.height);
        let closeness = area.abs_diff(1280 * 720);
        (pix, closeness, u32::MAX - area)
    };
    formats.iter().min_by_key(|fmt| ranked(fmt))
}

pub fn enumerate_sysfs(root: &Path) -> Vec<(PathBuf, String)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("video") {
            continue;
        }
        let dev = PathBuf::from("/dev").join(name.as_ref());
        let label = fs::read_to_string(entry.path().join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if label.is_empty() {
            continue;
        }
        found.push((dev, label));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

pub fn list_formats(device: &Path) -> Result<Vec<CameraFormat>, CaptureError> {
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-f",
            "v4l2",
            "-list_formats",
            "all",
            "-i",
        ])
        .arg(device.as_os_str())
        .output()
        .map_err(|err| CaptureError::Failed {
            what: "camera".into(),
            detail: err.to_string(),
        })?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let formats = parse_v4l2_list_formats(&text);
    if formats.is_empty() {
        Err(map_tool_text("camera", &text))
    } else {
        Ok(formats)
    }
}

pub fn enumerate_cameras() -> Vec<CameraDevice> {
    let mut cameras = Vec::new();
    for (path, name) in enumerate_sysfs(Path::new("/sys/class/video4linux")) {
        let formats = list_formats(&path).unwrap_or_default();
        if formats.is_empty() {
            continue;
        }
        cameras.push(CameraDevice {
            path: path.display().to_string(),
            name,
            formats,
        });
    }
    dedup_by_name(cameras)
}

fn dedup_by_name(cameras: Vec<CameraDevice>) -> Vec<CameraDevice> {
    let mut out: Vec<CameraDevice> = Vec::new();
    for camera in cameras {
        if let Some(existing) = out.iter_mut().find(|c| c.name == camera.name) {
            if camera.formats.len() > existing.formats.len() {
                *existing = camera;
            }
        } else {
            out.push(camera);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_mjpeg_near_720p() {
        let formats = vec![
            CameraFormat {
                pixel_format: "yuyv422".into(),
                width: 1920,
                height: 1080,
                fps: None,
            },
            CameraFormat {
                pixel_format: "mjpeg".into(),
                width: 1280,
                height: 720,
                fps: None,
            },
            CameraFormat {
                pixel_format: "mjpeg".into(),
                width: 640,
                height: 480,
                fps: None,
            },
        ];
        let best = prefer_format(&formats).unwrap();
        assert_eq!(best.pixel_format, "mjpeg");
        assert_eq!(best.width, 1280);
    }

    #[test]
    fn sysfs_scan_handles_missing_dir() {
        assert!(enumerate_sysfs(Path::new("/tmp/does-not-exist-scs")).is_empty());
    }

    #[test]
    fn label_includes_name_and_format() {
        let cam = CameraDevice {
            path: "/dev/video0".into(),
            name: "USB Camera".into(),
            formats: vec![CameraFormat {
                pixel_format: "mjpeg".into(),
                width: 1280,
                height: 720,
                fps: None,
            }],
        };
        assert_eq!(cam.label(), "USB Camera · 1280×720 · MJPEG");
    }
}
