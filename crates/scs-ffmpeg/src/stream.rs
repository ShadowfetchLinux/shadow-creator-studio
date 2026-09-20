use std::path::PathBuf;

use crate::builder::{FfmpegCommandBuilder, PlannedCommand};

pub const YOUTUBE_RTMP: &str = "rtmp://a.rtmp.youtube.com/live2";
pub const YOUTUBE_RTMPS: &str = "rtmps://a.rtmps.youtube.com/live2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub max_attempts: u32,
    pub delay_ms: u64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 8,
            delay_ms: 2_000,
        }
    }
}

impl ReconnectPolicy {
    pub fn next_delay_ms(self, attempt: u32) -> Option<u64> {
        if attempt >= self.max_attempts {
            None
        } else {
            Some(self.delay_ms.saturating_mul(1 + u64::from(attempt / 2)))
        }
    }
}

pub fn build_rtmp_url(server: &str, key: &str, rtmps: bool) -> Result<String, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("A stream key is required.".into());
    }
    if key.contains(char::is_whitespace) {
        return Err("The stream key must be a single token.".into());
    }
    let mut server = server.trim().trim_end_matches('/').to_string();
    if server.is_empty() {
        server = if rtmps {
            YOUTUBE_RTMPS.into()
        } else {
            YOUTUBE_RTMP.into()
        };
    }
    if !(server.starts_with("rtmp://") || server.starts_with("rtmps://")) {
        return Err("The ingest URL must start with rtmp:// or rtmps://.".into());
    }
    Ok(format!("{server}/{key}"))
}

pub fn redact_rtmp_url(url: &str) -> String {
    if let Some((base, _)) = url.rsplit_once('/') {
        format!("{base}/[redacted]")
    } else {
        "[redacted]".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProbe {
    pub ok: bool,
    pub reason: String,
    pub flv_muxer: bool,
}

pub fn evaluate_probe(key_present: bool, url: Result<&str, &str>, ffmpeg_muxers: &str) -> StreamProbe {
    let flv = ffmpeg_muxers.contains("flv");
    if !key_present {
        return StreamProbe {
            ok: false,
            reason: "GO LIVE stays off until a stream key is stored in the system keyring.".into(),
            flv_muxer: flv,
        };
    }
    if url.is_err() {
        return StreamProbe {
            ok: false,
            reason: "The ingest URL is not a valid RTMP/RTMPS target.".into(),
            flv_muxer: flv,
        };
    }
    if !flv {
        return StreamProbe {
            ok: false,
            reason: "FFmpeg on this machine does not list an flv muxer, so RTMP cannot start.".into(),
            flv_muxer: false,
        };
    }
    StreamProbe {
        ok: true,
        reason: "Key is stored and FFmpeg lists flv. Live can start.".into(),
        flv_muxer: true,
    }
}

pub fn plan_record_and_stream(
    local_mkv: impl Into<PathBuf>,
    rtmp_url: &str,
    video_bitrate: &str,
) -> PlannedCommand {
    let mkv = local_mkv.into();
    let tee = format!(
        "[f=matroska]{}|[f=flv]{rtmp_url}",
        mkv.display()
    );
    FfmpegCommandBuilder::new()
        .hide_banner()
        .arg("-nostats")
        .never_overwrite()
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("anullsrc=r=48000:cl=mono")
        .arg("-t")
        .arg("0")
        .arg("-b:v")
        .arg(video_bitrate)
        .arg("-f")
        .arg("tee")
        .arg("-map")
        .arg("0:a")
        .arg(tee)
        .build()
}

/// Live encode plan from an already-built record command: append tee output.
pub fn tee_outputs(local_mkv: &str, rtmp_url: &str) -> String {
    format!("[f=matroska]{local_mkv}|[f=flv]{rtmp_url}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_redacts_youtube_url() {
        let url = build_rtmp_url("", "xxxx-yyyy", false).unwrap();
        assert!(url.starts_with("rtmp://"));
        assert!(url.ends_with("/xxxx-yyyy"));
        assert_eq!(redact_rtmp_url(&url), format!("{YOUTUBE_RTMP}/[redacted]"));
        assert!(build_rtmp_url("https://evil", "k", false).is_err());
        assert!(build_rtmp_url("", "", false).is_err());
        let tls = build_rtmp_url("", "k", true).unwrap();
        assert!(tls.starts_with("rtmps://"));
    }

    #[test]
    fn reconnect_stops_after_max() {
        let policy = ReconnectPolicy {
            max_attempts: 2,
            delay_ms: 1000,
        };
        assert_eq!(policy.next_delay_ms(0), Some(1000));
        assert!(policy.next_delay_ms(2).is_none());
    }

    #[test]
    fn record_plus_stream_uses_tee() {
        let cmd = plan_record_and_stream("/tmp/a;rm.mkv", "rtmp://host/live/KEY", "6000k");
        let args: Vec<_> = cmd
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a.contains("tee")));
        assert!(args.iter().any(|a| a.contains("/tmp/a;rm.mkv")));
        assert!(args.contains(&"6000k".into()));
        assert_eq!(
            tee_outputs("/tmp/a.mkv", "rtmp://h/k"),
            "[f=matroska]/tmp/a.mkv|[f=flv]rtmp://h/k"
        );
    }

    #[test]
    fn probe_requires_key_and_flv() {
        assert!(!evaluate_probe(false, Ok("rtmp://a/b"), " E flv").ok);
        assert!(evaluate_probe(true, Ok("rtmp://a/b"), " E flv").ok);
        assert!(!evaluate_probe(true, Ok("rtmp://a/b"), "mov,mp4").ok);
    }
}
