use serde::{Deserialize, Serialize};

use crate::settings::HotkeySettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    StartStop,
    Pause,
    MuteMic,
    MuteDesktop,
    Marker,
    ToggleCamera,
    ToggleTeleprompter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

pub fn parse_hotkey(spec: &str) -> Result<ParsedHotkey, String> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut key = None;
    for part in spec.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            other => {
                if key.is_some() {
                    return Err(format!("Too many keys in '{spec}'"));
                }
                key = Some(normalize_key(other));
            }
        }
    }
    let key = key.ok_or_else(|| format!("No key in '{spec}'"))?;
    Ok(ParsedHotkey {
        key,
        ctrl,
        alt,
        shift,
    })
}

fn normalize_key(raw: &str) -> String {
    let upper = raw.to_ascii_uppercase();
    if upper.starts_with('F') && upper[1..].chars().all(|c| c.is_ascii_digit()) {
        return upper;
    }
    if raw.chars().count() == 1 {
        return raw.to_ascii_lowercase();
    }
    match raw.to_ascii_lowercase().as_str() {
        "space" => "space".into(),
        "escape" | "esc" => "escape".into(),
        other => other.to_string(),
    }
}

pub fn matches(spec: &str, key_name: &str, ctrl: bool, alt: bool, shift: bool) -> bool {
    parse_hotkey(spec)
        .map(|parsed| {
            parsed.key.eq_ignore_ascii_case(key_name)
                && parsed.ctrl == ctrl
                && parsed.alt == alt
                && parsed.shift == shift
        })
        .unwrap_or(false)
}

pub fn bindings(settings: &HotkeySettings) -> Vec<(HotkeyAction, String)> {
    vec![
        (HotkeyAction::StartStop, settings.start_stop.clone()),
        (HotkeyAction::Pause, settings.pause.clone()),
        (HotkeyAction::MuteMic, settings.mute_mic.clone()),
        (HotkeyAction::MuteDesktop, settings.mute_desktop.clone()),
        (HotkeyAction::Marker, settings.marker.clone()),
        (HotkeyAction::ToggleCamera, settings.toggle_camera.clone()),
        (
            HotkeyAction::ToggleTeleprompter,
            settings.toggle_teleprompter.clone(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_and_modifiers() {
        let key = parse_hotkey("Ctrl+Shift+F8").unwrap();
        assert_eq!(key.key, "F8");
        assert!(key.ctrl && key.shift && !key.alt);
        assert!(matches("F9", "F9", false, false, false));
        assert!(!matches("F9", "F9", true, false, false));
        assert!(parse_hotkey("").is_err());
        assert!(parse_hotkey("Ctrl+").is_err());
    }
}
