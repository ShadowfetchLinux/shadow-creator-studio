/// Mic monitoring is never wired into speakers. A loopback would feed back
/// on this desktop. The toggle is a reminder; no PipeWire loop is created.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorPolicy {
    pub enabled_pref: bool,
    pub will_loopback: bool,
    pub reason: &'static str,
}

pub fn monitor_policy(pref_enabled: bool, headphone_sink_present: bool) -> MonitorPolicy {
    if !pref_enabled {
        return MonitorPolicy {
            enabled_pref: false,
            will_loopback: false,
            reason: "Mic monitoring is off. The app does not create a speaker loopback.",
        };
    }
    if headphone_sink_present {
        MonitorPolicy {
            enabled_pref: true,
            will_loopback: false,
            reason: "Headphones look available, but this app still does not create a monitor loop. Use your desktop mixer if you need to hear yourself.",
        }
    } else {
        MonitorPolicy {
            enabled_pref: true,
            will_loopback: false,
            reason: "Monitoring stays off. A speaker loopback would feed back. Listen on headphones via the desktop mixer.",
        }
    }
}

pub fn looks_like_headphone(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("headphone") || lower.contains("headset") || lower.contains("earphone")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_creates_loopback() {
        let on = monitor_policy(true, true);
        assert!(!on.will_loopback);
        let speakers = monitor_policy(true, false);
        assert!(!speakers.will_loopback);
        assert!(looks_like_headphone("USB Headset"));
        assert!(!looks_like_headphone("Built-in Speakers"));
    }
}
