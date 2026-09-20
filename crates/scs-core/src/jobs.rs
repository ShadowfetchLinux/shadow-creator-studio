use std::path::{Path, PathBuf};

use crate::markers::MarkerFile;

#[derive(Debug, Clone, PartialEq)]
pub struct SilenceSpan {
    pub start_secs: f64,
    pub end_secs: f64,
}

pub fn chapters_from_markers(markers: &MarkerFile, dest: impl AsRef<Path>) -> Result<PathBuf, String> {
    let dest = dest.as_ref().to_path_buf();
    std::fs::write(&dest, markers.chapter_vtt()).map_err(|e| e.to_string())?;
    Ok(dest)
}

pub fn parse_silencedetect(log: &str) -> Vec<SilenceSpan> {
    let mut starts = Vec::new();
    let mut spans = Vec::new();
    for line in log.lines() {
        if let Some(v) = extract_after(line, "silence_start:") {
            starts.push(v);
        }
        if let Some(end) = extract_after(line, "silence_end:") {
            if let Some(start) = starts.pop() {
                spans.push(SilenceSpan {
                    start_secs: start,
                    end_secs: end,
                });
            }
        }
    }
    spans
}

fn extract_after(line: &str, key: &str) -> Option<f64> {
    let idx = line.find(key)?;
    let rest = line[idx + key.len()..].trim();
    let token = rest.split_whitespace().next()?;
    token.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::MarkerFile;

    #[test]
    fn writes_chapter_vtt() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = MarkerFile::new("rec");
        file.add(1_000, "Intro");
        file.add(5_000, "Hook");
        let dest = dir.path().join("chapters.vtt");
        chapters_from_markers(&file, &dest).unwrap();
        let text = std::fs::read_to_string(dest).unwrap();
        assert!(text.contains("WEBVTT"));
        assert!(text.contains("Intro"));
    }

    #[test]
    fn parses_silence_log() {
        let log = "[silencedetect @ 0] silence_start: 1.25\n[silencedetect @ 0] silence_end: 3.5 | silence_duration: 2.25\n";
        let spans = parse_silencedetect(log);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].start_secs - 1.25).abs() < 0.01);
        assert!((spans[0].end_secs - 3.5).abs() < 0.01);
    }
}
