use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl MemorySnapshot {
    pub fn used_bytes(self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }
}

pub fn parse_meminfo(text: &str) -> Option<MemorySnapshot> {
    let mut total_kb = None;
    let mut available_kb = None;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        let value: u64 = parts.next()?.parse().ok()?;
        match key {
            "MemTotal:" => total_kb = Some(value),
            "MemAvailable:" => available_kb = Some(value),
            _ => {}
        }
    }
    Some(MemorySnapshot {
        total_bytes: total_kb? * 1024,
        available_bytes: available_kb? * 1024,
    })
}

pub fn read_memory() -> Option<MemorySnapshot> {
    parse_meminfo(&fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo_fixture() {
        let text = "\
MemTotal:       65822764 kB
MemFree:         5800124 kB
MemAvailable:   46100112 kB
Buffers:          412000 kB
";
        let snap = parse_meminfo(text).unwrap();
        assert_eq!(snap.total_bytes, 65822764 * 1024);
        assert_eq!(snap.available_bytes, 46100112 * 1024);
        assert_eq!(snap.used_bytes(), snap.total_bytes - snap.available_bytes);
    }
}
