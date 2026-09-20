use serde::Deserialize;
use serde_json::Value;

use crate::error::PipewireError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwNode {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub nick: String,
    pub media_class: String,
    pub device_class: Option<String>,
    pub v4l2_path: Option<String>,
}

impl PwNode {
    pub fn display_name(&self) -> String {
        if !self.description.is_empty() {
            self.description.clone()
        } else if !self.nick.is_empty() {
            self.nick.clone()
        } else if !self.name.is_empty() {
            self.name.clone()
        } else {
            format!("Node {}", self.id)
        }
    }

    pub fn is_audio_source(&self) -> bool {
        self.media_class == "Audio/Source"
    }

    pub fn is_audio_sink(&self) -> bool {
        self.media_class == "Audio/Sink"
    }

    pub fn is_video_source(&self) -> bool {
        self.media_class == "Video/Source"
    }

    pub fn is_monitor_source(&self) -> bool {
        self.name.ends_with(".monitor")
            || self.device_class.as_deref() == Some("monitor")
            || self.description.to_ascii_lowercase().contains("monitor")
    }
}

#[derive(Debug, Deserialize)]
struct DumpNode {
    id: Option<u32>,
    #[serde(rename = "type")]
    node_type: Option<String>,
    info: Option<DumpInfo>,
}

#[derive(Debug, Deserialize)]
struct DumpInfo {
    props: Option<Value>,
}

/// Parse `pw-dump` JSON (array of objects) into node records.
pub fn parse_pw_dump(bytes: &[u8]) -> Result<Vec<PwNode>, PipewireError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|err| PipewireError::Parse(err.to_string()))?;
    let items = value
        .as_array()
        .ok_or_else(|| PipewireError::Parse("root is not an array".into()))?;
    let mut out = Vec::new();
    for item in items {
        let node: DumpNode = match serde_json::from_value(item.clone()) {
            Ok(n) => n,
            Err(_) => continue,
        };
        if node.node_type.as_deref() != Some("PipeWire:Interface:Node") {
            continue;
        }
        let Some(id) = node.id else { continue };
        let props = node
            .info
            .and_then(|info| info.props)
            .unwrap_or(Value::Null);
        let name = prop_string(&props, "node.name").unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        out.push(PwNode {
            id,
            description: prop_string(&props, "node.description").unwrap_or_default(),
            nick: prop_string(&props, "node.nick").unwrap_or_default(),
            media_class: prop_string(&props, "media.class").unwrap_or_default(),
            device_class: prop_string(&props, "device.class"),
            v4l2_path: prop_string(&props, "api.v4l2.path"),
            name,
        });
    }
    Ok(out)
}

fn prop_string(props: &Value, key: &str) -> Option<String> {
    props
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
pub(crate) fn tests_fixture() -> &'static [u8] {
    PW_DUMP_FIXTURE.as_bytes()
}

#[cfg(test)]
const PW_DUMP_FIXTURE: &str = r#"[
      {"id": 32, "type": "PipeWire:Interface:Core"},
      {
        "id": 40,
        "type": "PipeWire:Interface:Node",
        "info": {
          "props": {
            "node.name": "alsa_input.usb-generic_mic.mono",
            "node.description": "USB Microphone Analog Mono",
            "node.nick": "USB Microphone",
            "media.class": "Audio/Source"
          }
        }
      },
      {
        "id": 41,
        "type": "PipeWire:Interface:Node",
        "info": {
          "props": {
            "node.name": "alsa_output.usb-speakers.analog-stereo.monitor",
            "node.description": "USB Speakers Analog Stereo Monitor",
            "media.class": "Audio/Source",
            "device.class": "monitor"
          }
        }
      },
      {
        "id": 42,
        "type": "PipeWire:Interface:Node",
        "info": {
          "props": {
            "node.name": "alsa_output.usb-speakers.analog-stereo",
            "node.description": "USB Speakers Analog Stereo",
            "media.class": "Audio/Sink"
          }
        }
      },
      {
        "id": 50,
        "type": "PipeWire:Interface:Node",
        "info": {
          "props": {
            "node.name": "v4l2_input.usb-camera",
            "node.description": "USB Camera",
            "media.class": "Video/Source",
            "api.v4l2.path": "/dev/video0"
          }
        }
      }
    ]"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sources_sinks_and_cameras() {
        let nodes = parse_pw_dump(tests_fixture()).unwrap();
        assert_eq!(nodes.len(), 4);
        let mics: Vec<_> = nodes.iter().filter(|n| n.is_audio_source() && !n.is_monitor_source()).collect();
        let desks: Vec<_> = nodes.iter().filter(|n| n.is_monitor_source()).collect();
        assert_eq!(mics[0].display_name(), "USB Microphone Analog Mono");
        assert_eq!(desks[0].name, "alsa_output.usb-speakers.analog-stereo.monitor");
        assert_eq!(nodes.iter().find(|n| n.is_video_source()).unwrap().v4l2_path.as_deref(), Some("/dev/video0"));
    }

    #[test]
    fn rejects_non_array() {
        assert!(parse_pw_dump(b"{\"id\":1}").is_err());
    }
}
