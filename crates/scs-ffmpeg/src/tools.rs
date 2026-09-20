use std::path::{Path, PathBuf};

use crate::builder::{FfmpegCommandBuilder, PlannedCommand};

/// Quick-edit tools write a new file. They never overwrite the source (`-n`).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolJob {
    pub command: PlannedCommand,
    pub output: PathBuf,
}

fn dest(input: &Path, suffix: &str, ext: impl AsRef<str>) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "clip".into());
    input.with_file_name(format!("{stem}_{suffix}.{}", ext.as_ref()))
}

fn base(input: impl AsRef<Path>) -> FfmpegCommandBuilder {
    FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-nostats")
        .never_overwrite()
        .input(input.as_ref())
}

pub fn trim(input: impl AsRef<Path>, start_secs: f64, duration_secs: f64) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "trim", ext_of(input, "mkv"));
    ToolJob {
        command: base(input)
            .arg("-ss")
            .arg(format!("{start_secs:.3}"))
            .arg("-t")
            .arg(format!("{duration_secs:.3}"))
            .arg("-c")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

/// Remove a closed interval by concatenating the keep ranges. Re-encodes audio/video
/// with stream copy of the kept parts via the concat filter would need matching
/// codecs; we emit two `-ss/-to` copy segments into a concat list instead.
pub fn remove_section(
    input: impl AsRef<Path>,
    cut_start: f64,
    cut_end: f64,
    total_secs: f64,
) -> Result<ToolJob, String> {
    if cut_end <= cut_start {
        return Err("The removed section must end after it starts.".into());
    }
    if cut_start < 0.0 || cut_end > total_secs + 0.05 {
        return Err("The removed section is outside the clip.".into());
    }
    let input = input.as_ref();
    let output = dest(input, "cut", ext_of(input, "mkv"));
    let mut b = FfmpegCommandBuilder::new()
        .hide_banner()
        .never_overwrite()
        .arg("-ss")
        .arg("0")
        .arg("-to")
        .arg(format!("{cut_start:.3}"))
        .input(input)
        .arg("-ss")
        .arg(format!("{cut_end:.3}"))
        .input(input)
        .arg("-filter_complex")
        .arg("[0:v][0:a][1:v][1:a]concat=n=2:v=1:a=1[v][a]")
        .map_stream("[v]")
        .map_stream("[a]");
    b = b.output(&output);
    Ok(ToolJob {
        command: b.build(),
        output,
    })
}

pub fn normalize_audio(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "norm", ext_of(input, "mkv"));
    ToolJob {
        command: base(input)
            .arg("-af")
            .arg("loudnorm=I=-16:TP=-1.5:LRA=11")
            .arg("-c:v")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

pub fn change_resolution(input: impl AsRef<Path>, width: u32, height: u32) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, &format!("{width}x{height}"), ext_of(input, "mkv"));
    ToolJob {
        command: base(input)
            .arg("-vf")
            .arg(format!("scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2"))
            .arg("-c:a")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

pub fn compress(input: impl AsRef<Path>, crf: u8) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "small", ext_of(input, "mp4"));
    ToolJob {
        command: base(input)
            .arg("-c:v")
            .arg("libx264")
            .arg("-crf")
            .arg(crf.clamp(18, 32).to_string())
            .arg("-preset")
            .arg("medium")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("160k")
            .output(&output)
            .build(),
        output,
    }
}

pub fn extract_audio(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "audio", "m4a");
    ToolJob {
        command: base(input)
            .arg("-vn")
            .arg("-c:a")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

pub fn to_mp3(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "audio", "mp3");
    ToolJob {
        command: base(input)
            .arg("-vn")
            .arg("-c:a")
            .arg("libmp3lame")
            .arg("-q:a")
            .arg("2")
            .output(&output)
            .build(),
        output,
    }
}

pub fn to_wav(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "audio", "wav");
    ToolJob {
        command: base(input)
            .arg("-vn")
            .arg("-c:a")
            .arg("pcm_s16le")
            .output(&output)
            .build(),
        output,
    }
}

pub fn to_gif(input: impl AsRef<Path>, start_secs: f64, duration_secs: f64) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "clip", "gif");
    ToolJob {
        command: base(input)
            .arg("-ss")
            .arg(format!("{start_secs:.3}"))
            .arg("-t")
            .arg(format!("{duration_secs:.3}"))
            .arg("-vf")
            .arg("fps=12,scale=480:-1:flags=lanczos")
            .arg("-an")
            .output(&output)
            .build(),
        output,
    }
}

pub fn thumbnail(input: impl AsRef<Path>, at_secs: f64) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "thumb", "jpg");
    ToolJob {
        command: base(input)
            .arg("-ss")
            .arg(format!("{at_secs:.3}"))
            .arg("-frames:v")
            .arg("1")
            .arg("-q:v")
            .arg("3")
            .output(&output)
            .build(),
        output,
    }
}

pub fn remux_mp4(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "remux", "mp4");
    ToolJob {
        command: FfmpegCommandBuilder::remux_copy(input, &output),
        output,
    }
}

pub fn youtube_ready_mp4(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "youtube", "mp4");
    ToolJob {
        command: base(input)
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-profile:v")
            .arg("high")
            .arg("-level")
            .arg("4.2")
            .arg("-crf")
            .arg("18")
            .arg("-preset")
            .arg("medium")
            .arg("-movflags")
            .arg("+faststart")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k")
            .arg("-ar")
            .arg("48000")
            .output(&output)
            .build(),
        output,
    }
}

pub fn scale_9x16(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "9x16", ext_of(input, "mp4"));
    ToolJob {
        command: base(input)
            .arg("-vf")
            .arg("scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2")
            .arg("-c:a")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

pub fn probe_command(input: impl AsRef<Path>) -> PlannedCommand {
    PlannedCommand {
        program: PathBuf::from("ffprobe"),
        args: vec![
            "-v".into(),
            "error".into(),
            "-print_format".into(),
            "json".into(),
            "-show_format".into(),
            "-show_streams".into(),
            input.as_ref().as_os_str().to_os_string(),
        ],
    }
}

pub fn silence_remove(input: impl AsRef<Path>) -> ToolJob {
    let input = input.as_ref();
    let output = dest(input, "nosilence", ext_of(input, "mkv"));
    ToolJob {
        command: base(input)
            .arg("-af")
            .arg("silenceremove=stop_periods=-1:stop_duration=0.8:stop_threshold=-40dB")
            .arg("-c:v")
            .arg("copy")
            .output(&output)
            .build(),
        output,
    }
}

pub fn silence_detect_args(input: impl AsRef<Path>) -> PlannedCommand {
    FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-nostats")
        .input(input.as_ref())
        .arg("-af")
        .arg("silencedetect=n=-40dB:d=0.8")
        .arg("-f")
        .arg("null")
        .arg("-")
        .build()
}

fn ext_of(path: &Path, fallback: &str) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn args(job: &ToolJob) -> Vec<String> {
        job.command
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn tools_write_new_files_and_keep_paths_intact() {
        let nasty = PathBuf::from("/tmp/title;rm.mkv");
        let job = trim(&nasty, 1.5, 8.0);
        let a = args(&job);
        assert!(a.contains(&"-n".into()));
        assert!(a.contains(&"/tmp/title;rm.mkv".into()));
        assert!(job.output.to_string_lossy().contains("trim"));
        assert!(!a.iter().any(|x| *x == "rm"));
        let mp3 = to_mp3(&nasty);
        assert!(mp3.output.extension().unwrap() == "mp3");
        let yt = youtube_ready_mp4(&nasty);
        assert!(args(&yt).contains(&"+faststart".into()));
        assert!(remove_section(&nasty, 2.0, 1.0, 10.0).is_err());
        let cut = remove_section(&nasty, 2.0, 4.0, 10.0).unwrap();
        assert!(args(&cut).iter().any(|x| x.contains("concat")));
        let probe = probe_command(&nasty);
        assert_eq!(probe.program, PathBuf::from("ffprobe"));
        assert!(probe.args.iter().any(|a| a == "/tmp/title;rm.mkv"));
    }
}
