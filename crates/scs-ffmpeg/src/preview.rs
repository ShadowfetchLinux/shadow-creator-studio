use std::path::Path;

use crate::builder::{FfmpegCommandBuilder, PlannedCommand};

/// Low-latency RGB24 frames on stdout for the GTK preview.
pub fn camera_preview_rgb(
    device: impl AsRef<Path>,
    pixel_format: &str,
    width: u32,
    height: u32,
    fps: u32,
    mirror: bool,
) -> PlannedCommand {
    let size = format!("{width}x{height}");
    let fps = fps.max(1).to_string();
    let mut vf = format!("scale={width}:{height},format=rgb24,fps={fps}");
    if mirror {
        vf.push_str(",hflip");
    }
    let mut builder = FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-loglevel")
        .arg("error")
        .arg("-fflags")
        .arg("nobuffer")
        .arg("-flags")
        .arg("low_delay")
        .arg("-f")
        .arg("v4l2")
        .arg("-input_format")
        .arg(pixel_format)
        .arg("-framerate")
        .arg(&fps)
        .arg("-video_size")
        .arg(&size)
        .input(device.as_ref())
        .arg("-vf")
        .arg(&vf)
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24");
    builder = builder.arg("pipe:1");
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn preview_args_are_structured() {
        let nasty = "/dev/video0;rm";
        let cmd = camera_preview_rgb(nasty, "mjpeg", 640, 360, 15, true);
        assert_eq!(cmd.program, PathBuf::from("ffmpeg"));
        let args: Vec<String> = cmd
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&nasty.to_string()));
        assert!(args.contains(&"hflip".to_string()) || args.iter().any(|a| a.contains("hflip")));
        assert!(args.contains(&"mjpeg".to_string()));
        assert!(!args.iter().any(|a| *a == "rm"));
    }
}
