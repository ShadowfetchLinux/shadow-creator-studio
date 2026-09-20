use crate::meter::MeterLevels;

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationAdvice {
    pub recommended_volume: f32,
    pub suggested_preset: crate::chain::AudioProcessingPreset,
    pub note: String,
}

/// Target speaking peak around −12 dBFS. Never recommends silence or a blast.
pub fn recommend_from_peak(peak_db: f32) -> CalibrationAdvice {
    let target = -12.0;
    let delta = (target - peak_db).clamp(-12.0, 18.0);
    let volume = 10f32.powf(delta / 20.0).clamp(0.35, 2.2);
    let (preset, note) = if peak_db > -3.0 {
        (
            crate::chain::AudioProcessingPreset::QuietRoom,
            "You are very loud. Lower gain and keep Natural/Quiet Room light.",
        )
    } else if peak_db < -28.0 {
        (
            crate::chain::AudioProcessingPreset::NoisyRoom,
            "The signal is quiet. Speak closer or raise the recommended volume. Noise suppression stays moderate.",
        )
    } else {
        (
            crate::chain::AudioProcessingPreset::Natural,
            "Level looks usable. Natural processing stays light.",
        )
    };
    CalibrationAdvice {
        recommended_volume: (volume * 100.0).round() / 100.0,
        suggested_preset: preset,
        note: note.into(),
    }
}

pub fn recommend_from_levels(levels: MeterLevels) -> CalibrationAdvice {
    recommend_from_peak(levels.peak_db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meter::analyze_i16;

    #[test]
    fn hot_signal_lowers_gain() {
        let advice = recommend_from_peak(-1.0);
        assert!(advice.recommended_volume < 1.0);
        assert_eq!(
            advice.suggested_preset,
            crate::chain::AudioProcessingPreset::QuietRoom
        );
    }

    #[test]
    fn quiet_signal_raises_gain() {
        let advice = recommend_from_peak(-36.0);
        assert!(advice.recommended_volume > 1.0);
        assert!(advice.recommended_volume <= 2.2);
    }

    #[test]
    fn real_pcm_path() {
        let levels = analyze_i16(&[8_000, -8_000, 7_000]);
        let advice = recommend_from_levels(levels);
        assert!(advice.recommended_volume >= 0.35);
    }
}
