use crate::chain::AudioChain;
use crate::tracks::{TrackLayout, TrackRole};

/// FFmpeg `-filter_complex` using pad labels only — never interpolates device names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterGraph {
    pub graph: String,
    pub maps: Vec<String>,
}

pub fn build_filter_graph(
    chain: &AudioChain,
    layout: &TrackLayout,
    mic_index: Option<usize>,
    desktop_index: Option<usize>,
    music_index: Option<usize>,
) -> Option<FilterGraph> {
    let mut parts = Vec::new();
    let mut mix_inputs = Vec::new();

    if let Some(idx) = mic_index {
        let label = "mic";
        parts.push(format!(
            "[{idx}:a]{}[{label}]",
            mic_filters(chain, layout.microphone.effective_volume())
        ));
        if layout.microphone.enabled {
            mix_inputs.push(format!("[{label}]"));
        }
    }
    if let Some(idx) = desktop_index {
        if layout.desktop.audible() {
            parts.push(format!(
                "[{idx}:a]volume={}[desk]",
                fmt_vol(layout.desktop.effective_volume())
            ));
            mix_inputs.push("[desk]".into());
        }
    }
    if let Some(idx) = music_index {
        if layout.music.audible() {
            parts.push(format!(
                "[{idx}:a]volume={}[music]",
                fmt_vol(layout.music.effective_volume())
            ));
            mix_inputs.push("[music]".into());
        }
    }

    if parts.is_empty() {
        return None;
    }

    let mut maps = Vec::new();
    if layout.mixed.enabled && mix_inputs.len() >= 2 {
        parts.push(format!(
            "{}amix=inputs={}:duration=longest:dropout_transition=0[mix]",
            mix_inputs.join(""),
            mix_inputs.len()
        ));
        maps.push("[mix]".into());
    } else if layout.mixed.enabled && mix_inputs.len() == 1 {
        maps.push(mix_inputs[0].clone());
    }
    if layout.microphone.write_track && mic_index.is_some() {
        maps.push("[mic]".into());
    }
    if layout.desktop.write_track && layout.desktop.audible() && desktop_index.is_some() {
        maps.push("[desk]".into());
    }
    if layout.music.write_track && layout.music.audible() && music_index.is_some() {
        maps.push("[music]".into());
    }
    maps.dedup();
    Some(FilterGraph {
        graph: parts.join(";"),
        maps,
    })
}

fn mic_filters(chain: &AudioChain, volume: f32) -> String {
    let mut stages = vec![format!("volume={}", fmt_vol(volume))];
    if chain.high_pass.enabled {
        stages.push(format!("highpass=f={}", chain.high_pass.cutoff_hz.round()));
    }
    if chain.noise_suppression.enabled && chain.noise_suppression.strength > 0.0 {
        let nf = -50.0 + chain.noise_suppression.strength * 20.0;
        stages.push(format!("afftdn=nf={nf:.1}"));
    }
    if chain.gate.enabled {
        stages.push(format!(
            "agate=threshold={}dB:ratio=2:attack=5:release=80",
            chain.gate.threshold_db
        ));
    }
    if chain.eq.enabled {
        for band in &chain.eq.bands {
            stages.push(format!(
                "equalizer=f={}:t=q:w={}:g={}",
                band.freq_hz, band.q, band.gain_db
            ));
        }
    }
    if chain.compressor.enabled {
        stages.push(format!(
            "acompressor=threshold={}dB:ratio={}:attack={}:release={}",
            chain.compressor.threshold_db,
            chain.compressor.ratio,
            chain.compressor.attack_ms,
            chain.compressor.release_ms
        ));
    }
    if chain.limiter.enabled {
        stages.push(format!("alimiter=limit={}dB", chain.limiter.ceiling_db));
    }
    stages.join(",")
}

fn fmt_vol(volume: f32) -> String {
    format!("{:.3}", volume.clamp(0.0, 2.5))
}

pub fn default_roles() -> [TrackRole; 4] {
    [
        TrackRole::Mixed,
        TrackRole::Microphone,
        TrackRole::Desktop,
        TrackRole::Music,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::AudioChain;

    #[test]
    fn graph_uses_pads_not_device_names() {
        let mut layout = TrackLayout::default();
        layout.desktop.enabled = true;
        layout.desktop.source = Some("alsa_output.speakers;rm".into());
        let graph = build_filter_graph(&AudioChain::natural(), &layout, Some(1), Some(2), None)
            .unwrap();
        assert!(graph.graph.contains("[1:a]"));
        assert!(graph.graph.contains("[2:a]"));
        assert!(graph.graph.contains("highpass"));
        assert!(graph.graph.contains("[mix]"));
        assert!(!graph.graph.contains("rm"));
        assert!(graph.maps.contains(&"[mix]".into()));
        assert!(graph.maps.contains(&"[mic]".into()));
    }

    #[test]
    fn raw_still_applies_volume() {
        let layout = TrackLayout::default();
        let graph = build_filter_graph(&AudioChain::raw(), &layout, Some(0), None, None).unwrap();
        assert!(graph.graph.contains("volume="));
        assert!(!graph.graph.contains("highpass"));
    }
}
