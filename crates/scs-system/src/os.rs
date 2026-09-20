use std::fs;

pub fn os_pretty_name() -> Option<String> {
    parse_os_release(&fs::read_to_string("/etc/os-release").ok()?)
}

pub fn parse_os_release(text: &str) -> Option<String> {
    for line in text.lines() {
        let (key, value) = line.split_once('=')?;
        if key == "PRETTY_NAME" {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pretty_name() {
        let text = "NAME=\"Pop!_OS\"\nPRETTY_NAME=\"Pop!_OS 24.04 LTS\"\n";
        assert_eq!(
            parse_os_release(text).as_deref(),
            Some("Pop!_OS 24.04 LTS")
        );
    }
}
