use serde::{Deserialize, Serialize};

pub use scs_core::settings::AudioProcessingPreset;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighPassStage {
    pub enabled: bool,
    pub cutoff_hz: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseSuppressionStage {
    pub enabled: bool,
    /// 0.0–1.0. Natural uses a light value; never “maxed” by default.
    pub strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateStage {
    pub enabled: bool,
    pub threshold_db: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqStage {
    pub enabled: bool,
    pub bands: Vec<EqBand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorStage {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimiterStage {
    pub enabled: bool,
    pub ceiling_db: f32,
}

/// Mic → HPF → denoise → gate → EQ → compressor → limiter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioChain {
    pub preset: AudioProcessingPreset,
    pub high_pass: HighPassStage,
    pub noise_suppression: NoiseSuppressionStage,
    pub gate: GateStage,
    pub eq: EqStage,
    pub compressor: CompressorStage,
    pub limiter: LimiterStage,
}

impl AudioChain {
    pub fn raw() -> Self {
        Self {
            preset: AudioProcessingPreset::Raw,
            high_pass: HighPassStage {
                enabled: false,
                cutoff_hz: 80.0,
            },
            noise_suppression: NoiseSuppressionStage {
                enabled: false,
                strength: 0.0,
            },
            gate: GateStage {
                enabled: false,
                threshold_db: -48.0,
            },
            eq: EqStage {
                enabled: false,
                bands: Vec::new(),
            },
            compressor: CompressorStage {
                enabled: false,
                threshold_db: -18.0,
                ratio: 2.0,
                attack_ms: 15.0,
                release_ms: 80.0,
            },
            limiter: LimiterStage {
                enabled: false,
                ceiling_db: -1.0,
            },
        }
    }

    pub fn natural() -> Self {
        Self {
            preset: AudioProcessingPreset::Natural,
            high_pass: HighPassStage {
                enabled: true,
                cutoff_hz: 80.0,
            },
            noise_suppression: NoiseSuppressionStage {
                enabled: true,
                strength: 0.25,
            },
            gate: GateStage {
                enabled: false,
                threshold_db: -48.0,
            },
            eq: EqStage {
                enabled: true,
                bands: vec![EqBand {
                    freq_hz: 200.0,
                    gain_db: -1.0,
                    q: 0.7,
                }],
            },
            compressor: CompressorStage {
                enabled: true,
                threshold_db: -18.0,
                ratio: 2.0,
                attack_ms: 15.0,
                release_ms: 80.0,
            },
            limiter: LimiterStage {
                enabled: true,
                ceiling_db: -1.0,
            },
        }
    }

    pub fn for_preset(preset: AudioProcessingPreset) -> Self {
        match preset {
            AudioProcessingPreset::Raw => Self::raw(),
            AudioProcessingPreset::Natural => Self::natural(),
            AudioProcessingPreset::Voice | AudioProcessingPreset::Podcast => {
                let mut chain = Self::natural();
                chain.preset = preset;
                chain.gate.enabled = true;
                chain.noise_suppression.strength = 0.35;
                chain
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_is_light_not_raw() {
        let natural = AudioChain::natural();
        assert!(natural.high_pass.enabled);
        assert!(natural.noise_suppression.strength <= 0.35);
        assert!(!natural.gate.enabled);
        let raw = AudioChain::raw();
        assert!(!raw.high_pass.enabled);
        assert!(!raw.limiter.enabled);
    }

    #[test]
    fn chain_serde() {
        let json = serde_json::to_string(&AudioChain::natural()).unwrap();
        let back: AudioChain = serde_json::from_str(&json).unwrap();
        assert_eq!(back.preset, AudioProcessingPreset::Natural);
    }
}
