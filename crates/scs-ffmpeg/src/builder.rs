use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use scs_encoder::VideoEncoder;

/// A planned subprocess. Arguments stay as discrete `OsString`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl PlannedCommand {
    pub fn to_std(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command
    }
}

#[derive(Debug, Clone)]
pub struct FfmpegCommandBuilder {
    program: PathBuf,
    args: Vec<OsString>,
}

impl Default for FfmpegCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FfmpegCommandBuilder {
    pub fn new() -> Self {
        Self {
            program: PathBuf::from("ffmpeg"),
            args: Vec::new(),
        }
    }

    pub fn program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = program.into();
        self
    }

    pub fn arg(mut self, value: impl AsRef<OsStr>) -> Self {
        self.args.push(value.as_ref().to_os_string());
        self
    }

    pub fn hide_banner(self) -> Self {
        self.arg("-hide_banner")
    }

    /// `-n` — never overwrite the output path.
    pub fn never_overwrite(self) -> Self {
        self.arg("-n")
    }

    pub fn input(self, spec: impl AsRef<OsStr>) -> Self {
        self.arg("-i").arg(spec)
    }

    pub fn video_codec(self, encoder: VideoEncoder) -> Self {
        self.arg("-c:v").arg(encoder.ffmpeg_name())
    }

    pub fn audio_codec_aac(self) -> Self {
        self.arg("-c:a").arg("aac")
    }

    pub fn map_stream(self, spec: &str) -> Self {
        self.arg("-map").arg(spec)
    }

    pub fn output(self, path: impl AsRef<Path>) -> Self {
        self.arg(path.as_ref().as_os_str())
    }

    pub fn remux_copy(input: impl AsRef<Path>, output: impl AsRef<Path>) -> PlannedCommand {
        Self::new()
            .hide_banner()
            .never_overwrite()
            .input(input.as_ref())
            .arg("-c")
            .arg("copy")
            .output(output)
            .build()
    }

    pub fn record_stub(
        input: impl AsRef<OsStr>,
        encoder: VideoEncoder,
        output: impl AsRef<Path>,
    ) -> PlannedCommand {
        Self::new()
            .hide_banner()
            .never_overwrite()
            .input(input)
            .video_codec(encoder)
            .audio_codec_aac()
            .output(output)
            .build()
    }

    pub fn build(self) -> PlannedCommand {
        PlannedCommand {
            program: self.program,
            args: self.args,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemuxPlan {
    pub input: PathBuf,
    pub output: PathBuf,
}

impl RemuxPlan {
    pub fn command(&self) -> PlannedCommand {
        FfmpegCommandBuilder::remux_copy(&self.input, &self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remux_uses_copy_and_never_overwrite() {
        let cmd = FfmpegCommandBuilder::remux_copy("/tmp/in.mkv", "/tmp/out.mp4");
        assert_eq!(cmd.program, PathBuf::from("ffmpeg"));
        let args: Vec<String> = cmd
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "-hide_banner",
                "-n",
                "-i",
                "/tmp/in.mkv",
                "-c",
                "copy",
                "/tmp/out.mp4"
            ]
        );
    }

    #[test]
    fn user_path_is_single_argv_not_interpolated() {
        let nasty = "/tmp/title; rm -rf / && echo.mkv";
        let cmd = FfmpegCommandBuilder::record_stub(
            "pipewire",
            VideoEncoder::H264Nvenc,
            nasty,
        );
        let args: Vec<String> = cmd
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&nasty.to_string()));
        assert_eq!(args.iter().filter(|a| a.contains(';')).count(), 1);
        assert!(args.contains(&"h264_nvenc".to_string()));
        assert!(!args.iter().any(|a| *a == "rm" || *a == "-rf"));
    }

    #[test]
    fn to_std_sets_program_and_args() {
        let cmd = FfmpegCommandBuilder::new().arg("-version").build();
        let std = cmd.to_std();
        assert_eq!(std.get_program(), OsStr::new("ffmpeg"));
        let collected: Vec<_> = std.get_args().collect();
        assert_eq!(collected, vec![OsStr::new("-version")]);
    }
}
