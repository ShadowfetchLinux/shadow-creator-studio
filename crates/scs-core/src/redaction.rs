use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

fn secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase().replace('-', "_");
    k == "password"
        || k == "passwd"
        || k == "secret"
        || k == "token"
        || k == "access_token"
        || k == "refresh_token"
        || k == "stream_key"
        || k == "streamkey"
        || k == "authorization"
        || k == "api_key"
        || k == "apikey"
        || k == "private_key"
        || k == "client_secret"
        || k == "obs_websocket_password"
        || k == "credential"
        || k == "credentials"
        || k == "auth"
        || k == "bearer"
        || k.contains("password")
        || k.contains("stream_key")
        || k.contains("api_key")
        || k.ends_with("_token")
        || k.ends_with("_secret")
}

/// Recursively replace secret object fields with `[redacted]`.
pub fn redact_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if secret_key(&key) {
                    map.insert(key, Value::String("[redacted]".into()));
                } else if let Some(child) = map.get_mut(&key) {
                    redact_json(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json(item);
            }
        }
        _ => {}
    }
}

fn patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._\-+=/]{8,}").expect("bearer"),
            Regex::new(r"(?i)((?:stream[_-]?key|password|token|api[_-]?key|secret|authorization)\s*[:=]\s*)\S+").expect("kv"),
            Regex::new(r"(?i)(xox[baprs]-)[A-Za-z0-9-]{8,}").expect("slack"),
        ]
    })
}

/// Redact JSON if the text is JSON; otherwise apply conservative token patterns.
pub fn redact_text(input: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<Value>(input) {
        redact_json(&mut value);
        return serde_json::to_string_pretty(&value).unwrap_or_else(|_| fallback(input));
    }
    fallback(input)
}

fn fallback(input: &str) -> String {
    let mut out = input.to_string();
    for re in patterns() {
        out = re.replace_all(&out, "$1[redacted]").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_nested_secrets() {
        let mut value = json!({
            "os": "Pop!_OS",
            "stream_key": "live_should_not_leak",
            "nested": {
                "password": "hunter2",
                "apiKey": "abc123456"
            },
            "harmless": "1080p60"
        });
        redact_json(&mut value);
        assert_eq!(value["stream_key"], "[redacted]");
        assert_eq!(value["nested"]["password"], "[redacted]");
        assert_eq!(value["nested"]["apiKey"], "[redacted]");
        assert_eq!(value["harmless"], "1080p60");
        assert_eq!(value["os"], "Pop!_OS");
    }

    #[test]
    fn redacts_text_tokens() {
        let text = "Authorization: Bearer SUPERSECRETTOKEN\nstream_key=yt-live-xxxx\n";
        let redacted = redact_text(text);
        assert!(!redacted.contains("SUPERSECRETTOKEN"), "{redacted}");
        assert!(!redacted.contains("yt-live-xxxx"), "{redacted}");
        assert!(redacted.contains("[redacted]"));
    }

    #[test]
    fn json_text_path() {
        let redacted = redact_text(r#"{"token":"abc","fps":60}"#);
        assert!(redacted.contains("[redacted]"));
        assert!(redacted.contains("60"));
        assert!(!redacted.contains("abc"));
    }
}
