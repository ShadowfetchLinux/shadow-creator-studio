use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSample {
    pub idle: u64,
    pub total: u64,
}

impl CpuSample {
    pub fn percent_since(self, next: CpuSample) -> Option<f32> {
        let idle = next.idle.saturating_sub(self.idle);
        let total = next.total.saturating_sub(self.total);
        if total == 0 {
            return None;
        }
        Some((1.0 - idle as f32 / total as f32) * 100.0)
    }
}

/// First `cpu ` line of `/proc/stat`.
pub fn parse_proc_stat_cpu_line(line: &str) -> Option<CpuSample> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "cpu" {
        return None;
    }
    let mut values = [0u64; 10];
    let mut count = 0;
    for (slot, token) in values.iter_mut().zip(parts) {
        *slot = token.parse().ok()?;
        count += 1;
    }
    if count < 4 {
        return None;
    }
    let user = values[0];
    let nice = values[1];
    let system = values[2];
    let idle = values[3];
    let iowait = values[4];
    let irq = values[5];
    let softirq = values[6];
    let steal = values[7];
    let total = user
        .saturating_add(nice)
        .saturating_add(system)
        .saturating_add(idle)
        .saturating_add(iowait)
        .saturating_add(irq)
        .saturating_add(softirq)
        .saturating_add(steal);
    let idle_all = idle.saturating_add(iowait);
    Some(CpuSample {
        idle: idle_all,
        total,
    })
}

pub fn read_cpu_sample() -> Option<CpuSample> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().next()?;
    parse_proc_stat_cpu_line(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_stat_fixture() {
        let line = "cpu  4705324 772 1172263 18501504 2044 0 312 0 0 0";
        let sample = parse_proc_stat_cpu_line(line).unwrap();
        assert_eq!(sample.idle, 18501504 + 2044);
        assert!(sample.total > sample.idle);
    }

    #[test]
    fn percent_between_samples() {
        let a = CpuSample {
            idle: 100,
            total: 200,
        };
        let b = CpuSample {
            idle: 140,
            total: 300,
        };
        let pct = a.percent_since(b).unwrap();
        assert!((pct - 60.0).abs() < 0.01);
    }

    #[test]
    fn rejects_short_line() {
        assert!(parse_proc_stat_cpu_line("cpu 1 2").is_none());
    }
}
