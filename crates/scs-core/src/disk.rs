/// Filesystem capacity used for recording estimates. Values are bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpace {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl DiskSpace {
    /// Leave 1 GiB free so the volume does not fill during a wrap-up remux.
    pub const RESERVE_BYTES: u64 = 1 << 30;

    pub fn from_blocks(fragment_size: u64, blocks: u64, available_blocks: u64) -> Self {
        Self {
            total_bytes: fragment_size.saturating_mul(blocks),
            available_bytes: fragment_size.saturating_mul(available_blocks),
        }
    }

    pub fn used_bytes(self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_ratio(self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.used_bytes() as f64 / self.total_bytes as f64
        }
    }

    pub fn usable_bytes(self) -> u64 {
        self.available_bytes.saturating_sub(Self::RESERVE_BYTES)
    }

    /// `None` only when bitrate is zero. Insufficient free space yields `Some(0)`.
    pub fn estimate_seconds(self, bitrate_bps: u64) -> Option<u64> {
        if bitrate_bps == 0 {
            return None;
        }
        Some(self.usable_bytes().saturating_mul(8) / bitrate_bps)
    }

    pub fn bytes_for_duration(duration_secs: u64, bitrate_bps: u64) -> u64 {
        duration_secs.saturating_mul(bitrate_bps) / 8
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    let b = bytes as f64;
    if b >= TIB {
        format!("{:.2} TiB", b / TIB)
    } else if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_duration_seconds(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_blocks_and_used() {
        let space = DiskSpace::from_blocks(4096, 1_000_000, 250_000);
        assert_eq!(space.total_bytes, 4096 * 1_000_000);
        assert_eq!(space.available_bytes, 4096 * 250_000);
        assert_eq!(space.used_bytes(), 4096 * 750_000);
        assert!((space.used_ratio() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_accounts_for_reserve() {
        let space = DiskSpace {
            total_bytes: 20 * DiskSpace::RESERVE_BYTES,
            available_bytes: 2 * DiskSpace::RESERVE_BYTES,
        };
        assert_eq!(space.usable_bytes(), DiskSpace::RESERVE_BYTES);
        let secs = space.estimate_seconds(8_000_000).unwrap();
        assert_eq!(secs, (DiskSpace::RESERVE_BYTES * 8) / 8_000_000);
    }

    #[test]
    fn zero_bitrate_is_none_low_space_is_zero() {
        let space = DiskSpace {
            total_bytes: 100,
            available_bytes: 50,
        };
        assert_eq!(space.estimate_seconds(0), None);
        assert_eq!(space.estimate_seconds(8_000_000), Some(0));
    }

    #[test]
    fn bytes_for_duration() {
        assert_eq!(DiskSpace::bytes_for_duration(10, 8_000_000), 10_000_000);
    }

    #[test]
    fn formatters() {
        assert_eq!(format_bytes(512), "512 B");
        assert!(format_bytes(DiskSpace::RESERVE_BYTES).contains("GiB"));
        assert_eq!(format_duration_seconds(90), "1m");
        assert_eq!(format_duration_seconds(3661), "1h 01m");
    }
}
