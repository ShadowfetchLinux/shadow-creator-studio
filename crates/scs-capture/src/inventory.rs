use scs_audio::AudioDevice;
use scs_pipewire::{audio_devices_from_nodes, dump_nodes};

use crate::error::CaptureError;
use crate::v4l2::{enumerate_cameras, CameraDevice};

#[derive(Debug, Clone, Default)]
pub struct DeviceInventory {
    pub cameras: Vec<CameraDevice>,
    pub microphones: Vec<AudioDevice>,
    pub desktop_audio: Vec<AudioDevice>,
    pub errors: Vec<String>,
}

impl DeviceInventory {
    pub fn discover() -> Self {
        let mut inventory = Self::default();
        inventory.cameras = enumerate_cameras();
        if inventory.cameras.is_empty() {
            inventory.errors.push(
                CaptureError::NotFound {
                    what: "camera".into(),
                }
                .human_message(),
            );
        }
        match dump_nodes() {
            Ok(nodes) => {
                let (mics, desktop) = audio_devices_from_nodes(&nodes);
                if mics.is_empty() {
                    inventory.errors.push(
                        CaptureError::NotFound {
                            what: "microphone".into(),
                        }
                        .human_message(),
                    );
                }
                if desktop.is_empty() {
                    inventory.errors.push(
                        CaptureError::NotFound {
                            what: "desktop audio monitor".into(),
                        }
                        .human_message(),
                    );
                }
                inventory.microphones = mics;
                inventory.desktop_audio = desktop;
            }
            Err(err) => inventory.errors.push(err.human_message()),
        }
        inventory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scs_audio::AudioDeviceKind;

    #[test]
    fn default_inventory_is_empty() {
        let inventory = DeviceInventory::default();
        assert!(inventory.cameras.is_empty());
        assert!(inventory.microphones.is_empty());
        let _ = AudioDeviceKind::Microphone;
    }
}
