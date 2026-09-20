use crate::v4l2::CameraFormat;

/// Parse `ffmpeg -f v4l2 -list_formats all -i /dev/videoN` stderr.
pub fn parse_v4l2_list_formats(text: &str) -> Vec<CameraFormat> {
    let mut out = Vec::new();
    for line in text.lines() {
        if !line.contains("video4linux") {
            continue;
        }
        let Some((_, rest)) = line.split_once(']') else {
            continue;
        };
        let Some((_, after_kind)) = rest.split_once(':') else {
            continue;
        };
        let Some((pix, sizes)) = after_kind.split_once(':') else {
            continue;
        };
        let pix = pix.trim();
        if pix.is_empty() || !pix.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        for token in sizes.split_whitespace() {
            if let Some((width, height)) = parse_size(token) {
                out.push(CameraFormat {
                    pixel_format: pix.to_ascii_lowercase(),
                    width,
                    height,
                    fps: None,
                });
            }
        }
    }
    out
}

fn parse_size(token: &str) -> Option<(u32, u32)> {
    let (w, h) = token.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
[video4linux2,v4l2 @ 0x1] Raw       :     yuyv422 :           YUYV 4:2:2 : 640x480 160x120 1280x720 1920x1080
[video4linux2,v4l2 @ 0x1] Compressed:       mjpeg :          Motion-JPEG : 640x480 1280x720 1920x1080
[in#0 @ 0x1] Error opening input: Immediate exit requested
";

    #[test]
    fn parses_mjpeg_and_yuyv_sizes() {
        let formats = parse_v4l2_list_formats(FIXTURE);
        assert!(formats
            .iter()
            .any(|f| f.pixel_format == "mjpeg" && f.width == 1920));
        assert!(formats
            .iter()
            .any(|f| f.pixel_format == "yuyv422" && f.height == 720));
        assert_eq!(formats.len(), 7);
    }
}
