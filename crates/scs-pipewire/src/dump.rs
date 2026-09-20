use std::process::Command;

use scs_audio::{AudioDevice, AudioDeviceKind};

use crate::error::{map_command_failure, map_spawn_error, PipewireError};
use crate::nodes::{parse_pw_dump, PwNode};
use crate::{detect, PipewireStatus};

pub fn dump_nodes() -> Result<Vec<PwNode>, PipewireError> {
    if matches!(detect(), PipewireStatus::Missing { .. }) {
        return Err(PipewireError::NotRunning);
    }
    let output = Command::new("pw-dump")
        .output()
        .map_err(|err| map_spawn_error("pw-dump", &err))?;
    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(map_command_failure(
            "pw-dump",
            &stderr,
            output.status.code(),
        ));
    }
    parse_pw_dump(&output.stdout)
}

pub fn audio_devices_from_nodes(nodes: &[PwNode]) -> (Vec<AudioDevice>, Vec<AudioDevice>) {
    let mut mics = Vec::new();
    let mut desktop = Vec::new();
    for node in nodes {
        if !node.is_audio_source() {
            continue;
        }
        let device = AudioDevice {
            id: node.name.clone(),
            name: node.display_name(),
            kind: if node.is_monitor_source() {
                AudioDeviceKind::Desktop
            } else {
                AudioDeviceKind::Microphone
            },
        };
        if node.is_monitor_source() {
            desktop.push(device);
        } else {
            mics.push(device);
        }
    }
    if desktop.is_empty() {
        for node in nodes.iter().filter(|n| n.is_audio_sink()) {
            desktop.push(AudioDevice {
                id: format!("{}.monitor", node.name),
                name: format!("{} (monitor)", node.display_name()),
                kind: AudioDeviceKind::Desktop,
            });
        }
    }
    (mics, desktop)
}

/// Structured argv for `pw-record` raw s16le capture. Target is a single argument.
pub fn pw_record_args(target: &str) -> Vec<String> {
    vec![
        "--target".into(),
        target.into(),
        "--rate".into(),
        "48000".into(),
        "--channels".into(),
        "1".into(),
        "--format".into(),
        "s16".into(),
        "--latency".into(),
        "50ms".into(),
        "-a".into(),
        "-".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_mic_and_desktop() {
        let nodes = parse_pw_dump(crate::nodes::tests_fixture()).unwrap();
        let (mics, desktop) = audio_devices_from_nodes(&nodes);
        assert_eq!(mics.len(), 1);
        assert_eq!(desktop.len(), 1);
        assert_eq!(mics[0].kind, AudioDeviceKind::Microphone);
        assert_eq!(desktop[0].kind, AudioDeviceKind::Desktop);
    }

    #[test]
    fn record_args_keep_target_intact() {
        let target = "alsa_input.usb-generic;rm -rf";
        let args = pw_record_args(target);
        assert_eq!(args[1], target);
        assert!(!args.iter().any(|a| a == "rm"));
    }
}
