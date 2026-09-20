#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FfmpegProgress {
    pub frame: u64,
    pub out_time_ms: u64,
    pub drop_frames: Option<u64>,
    pub speed: Option<String>,
    pub finished: bool,
}

pub fn parse_progress_block(text: &str) -> FfmpegProgress {
    let mut progress = FfmpegProgress::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "frame" => progress.frame = value.trim().parse().unwrap_or(progress.frame),
            "out_time_ms" => {
                progress.out_time_ms = value.trim().parse().unwrap_or(progress.out_time_ms)
            }
            "out_time_us" => {
                if let Ok(us) = value.trim().parse::<u64>() {
                    progress.out_time_ms = us / 1000;
                }
            }
            "drop_frames" => progress.drop_frames = value.trim().parse().ok(),
            "speed" => {
                let trimmed = value.trim();
                if !trimmed.is_empty() && trimmed != "N/A" {
                    progress.speed = Some(trimmed.to_string());
                }
            }
            "progress" => progress.finished = value.trim() == "end",
            _ => {}
        }
    }
    progress
}

pub fn parse_drop_from_stats(line: &str) -> Option<u64> {
    let drop = line.split("drop=").nth(1)?;
    let token = drop.split_whitespace().next()?;
    token.parse().ok()
}

pub fn human_ffmpeg_error(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if lower.contains("no space") || lower.contains("no space left") {
        "The disk filled while writing the take. The MKV that exists is kept.".into()
    } else if lower.contains("busy") {
        "A capture device is busy in another application.".into()
    } else if lower.contains("permission denied") {
        "Cannot open a capture device — permission denied.".into()
    } else if lower.contains("no such file") || lower.contains("not found") {
        "A capture device disappeared.".into()
    } else if lower.contains("cannot allocate") || lower.contains("out of memory") {
        "FFmpeg ran out of memory.".into()
    } else {
        let snippet = text
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("FFmpeg stopped unexpectedly");
        format!(
            "Recording stopped: {}",
            snippet.chars().take(180).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_and_end() {
        let block = "\
frame=120
out_time_ms=4000
drop_frames=3
speed=1.01x
progress=end
";
        let parsed = parse_progress_block(block);
        assert_eq!(parsed.frame, 120);
        assert_eq!(parsed.out_time_ms, 4000);
        assert_eq!(parsed.drop_frames, Some(3));
        assert!(parsed.finished);
    }

    #[test]
    fn maps_disk_full() {
        assert!(human_ffmpeg_error("os error: No space left on device").contains("disk"));
    }

    #[test]
    fn stats_drop_count() {
        assert_eq!(
            parse_drop_from_stats("frame=  10 fps=30 q=23.0 size=  100kB time=00:00:01.00 bitrate= 800.0kbits/s drop=4 speed=1.00x"),
            Some(4)
        );
    }
}
