/// Peak / average levels from linear PCM. Values are 0.0–1.0 unless noted as dB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterLevels {
    pub peak: f32,
    pub average: f32,
    pub peak_db: f32,
    pub average_db: f32,
    pub clipping: bool,
}

impl MeterLevels {
    pub const CLIP_LINEAR: f32 = 0.99;
    pub const FLOOR_DB: f32 = -90.0;
    pub const METER_FLOOR_DB: f32 = -60.0;

    pub fn silence() -> Self {
        Self {
            peak: 0.0,
            average: 0.0,
            peak_db: Self::FLOOR_DB,
            average_db: Self::FLOOR_DB,
            clipping: false,
        }
    }
}

pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 1.0e-9 {
        MeterLevels::FLOOR_DB
    } else {
        20.0 * linear.log10()
    }
}

pub fn is_clipping(peak_linear: f32) -> bool {
    peak_linear >= MeterLevels::CLIP_LINEAR
}

/// Map dBFS in [-60, 0] onto a 0–1 meter bar.
pub fn meter_fraction(peak_db: f32) -> f64 {
    let span = -MeterLevels::METER_FLOOR_DB;
    ((peak_db - MeterLevels::METER_FLOOR_DB) / span).clamp(0.0, 1.0) as f64
}

pub fn analyze_i16(samples: &[i16]) -> MeterLevels {
    if samples.is_empty() {
        return MeterLevels::silence();
    }
    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f32;
    for sample in samples {
        let linear = (*sample as f32) / 32768.0;
        let abs = linear.abs();
        if abs > peak {
            peak = abs;
        }
        sum_sq += linear * linear;
    }
    let average = (sum_sq / samples.len() as f32).sqrt();
    MeterLevels {
        peak,
        average,
        peak_db: linear_to_db(peak),
        average_db: linear_to_db(average),
        clipping: is_clipping(peak),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_and_full_scale() {
        let quiet = analyze_i16(&[0, 0, 0, 0]);
        assert!(!quiet.clipping);
        assert!(quiet.peak < 0.01);
        assert!(meter_fraction(quiet.peak_db) < 0.05);

        let hot = analyze_i16(&[32767, -32767, 30000]);
        assert!(hot.clipping);
        assert!(hot.peak > 0.99);
        assert!(meter_fraction(0.0) > 0.99);
        assert!((meter_fraction(-30.0) - 0.5).abs() < 0.02);
    }

    #[test]
    fn empty_is_silence() {
        assert_eq!(analyze_i16(&[]).peak_db, MeterLevels::FLOOR_DB);
        assert!(!is_clipping(0.5));
        assert!(is_clipping(1.0));
    }
}
