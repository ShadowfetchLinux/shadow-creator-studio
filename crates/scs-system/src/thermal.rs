use std::fs;
use std::path::Path;

/// Prefer AMD `k10temp`, then the first hwmon that is not Wi-Fi or a battery.
pub fn cpu_temperature_c() -> Option<f32> {
    let hwmon = Path::new("/sys/class/hwmon");
    let entries = fs::read_dir(hwmon).ok()?;
    let mut fallback = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = fs::read_to_string(dir.join("name")).unwrap_or_default();
        let name = name.trim();
        let temp = read_temp_input(&dir.join("temp1_input"))?;
        if name == "k10temp" {
            return Some(temp);
        }
        if name.starts_with("coretemp") {
            return Some(temp);
        }
        if fallback.is_none()
            && !name.starts_with("iwlwifi")
            && !name.starts_with("hidpp")
            && name != "nvme"
        {
            fallback = Some(temp);
        }
    }
    fallback
}

fn read_temp_input(path: &Path) -> Option<f32> {
    let raw = fs::read_to_string(path).ok()?;
    parse_hwmon_temp_millidegree(raw.trim())
}

pub fn parse_hwmon_temp_millidegree(text: &str) -> Option<f32> {
    let milli: f32 = text.parse().ok()?;
    Some(milli / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millidegree_parse() {
        assert_eq!(parse_hwmon_temp_millidegree("45250"), Some(45.25));
    }
}
