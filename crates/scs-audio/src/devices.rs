use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDeviceKind {
    Microphone,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub kind: AudioDeviceKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_serde() {
        let device = AudioDevice {
            id: "alsa_input.usb".into(),
            name: "USB Microphone".into(),
            kind: AudioDeviceKind::Microphone,
        };
        let json = serde_json::to_string(&device).unwrap();
        let back: AudioDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, AudioDeviceKind::Microphone);
    }
}
