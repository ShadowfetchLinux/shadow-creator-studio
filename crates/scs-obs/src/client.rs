use std::net::TcpStream;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

use crate::ObsConnectionConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObsEngineStatus {
    Connected { version: String },
    AuthRequired,
    Unreachable { reason: String },
}

#[derive(Debug, Deserialize)]
struct HelloMsg {
    op: u32,
    d: HelloData,
}

#[derive(Debug, Deserialize)]
struct HelloData {
    #[serde(rename = "obsWebSocketVersion")]
    obs_websocket_version: Option<String>,
    #[serde(rename = "rpcVersion")]
    rpc_version: Option<u32>,
    authentication: Option<AuthChallenge>,
}

#[derive(Debug, Deserialize)]
struct AuthChallenge {
    challenge: String,
    salt: String,
}

#[derive(Debug, Serialize)]
struct IdentifyMsg {
    op: u32,
    d: IdentifyData,
}

#[derive(Debug, Serialize)]
struct IdentifyData {
    #[serde(rename = "rpcVersion")]
    rpc_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication: Option<String>,
}

#[derive(Debug, Serialize)]
struct RequestMsg {
    op: u32,
    d: RequestData,
}

#[derive(Debug, Serialize)]
struct RequestData {
    #[serde(rename = "requestType")]
    request_type: String,
    #[serde(rename = "requestId")]
    request_id: String,
}

/// OBS WebSocket 5 authentication string. Never log the password.
pub fn auth_string(password: &str, salt: &str, challenge: &str) -> String {
    let mut secret_hash = Sha256::new();
    secret_hash.update(password.as_bytes());
    secret_hash.update(salt.as_bytes());
    let secret = STANDARD.encode(secret_hash.finalize());
    let mut auth_hash = Sha256::new();
    auth_hash.update(secret.as_bytes());
    auth_hash.update(challenge.as_bytes());
    STANDARD.encode(auth_hash.finalize())
}

pub fn probe(cfg: &ObsConnectionConfig) -> ObsEngineStatus {
    match TcpStream::connect_timeout(&cfg.socket_addr(), Duration::from_millis(250)) {
        Ok(_) => ObsEngineStatus::Unreachable {
            reason: "Port is open. Open a WebSocket session to finish Identify.".into(),
        },
        Err(err) => ObsEngineStatus::Unreachable {
            reason: format!("OBS WebSocket is not reachable on {}:{} ({err})", cfg.host, cfg.port),
        },
    }
}

impl ObsConnectionConfig {
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        let ip = self
            .host
            .parse::<IpAddr>()
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        SocketAddr::new(ip, self.port)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", self.host, self.port)
    }
}

/// Blocking OBS WebSocket v5 session. Used from a worker thread.
pub struct ObsSession {
    socket: tungstenite::WebSocket<TcpStream>,
    next_id: u32,
}

impl ObsSession {
    pub fn connect(cfg: &ObsConnectionConfig, password: Option<&str>) -> Result<Self, String> {
        let stream = TcpStream::connect_timeout(&cfg.socket_addr(), Duration::from_secs(2))
            .map_err(|e| {
                format!(
                    "OBS is not listening on {}:{} ({e}). Start OBS with WebSocket enabled, or use FFmpeg/GStreamer.",
                    cfg.host, cfg.port
                )
            })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .ok();
        stream.set_write_timeout(Some(Duration::from_secs(8))).ok();
        let url = cfg.ws_url();
        let (mut socket, _) = tungstenite::client::client(&url, stream)
            .map_err(|e| format!("OBS WebSocket handshake failed: {e}"))?;

        let hello = read_json(&mut socket)?;
        let hello: HelloMsg = serde_json::from_value(hello)
            .map_err(|_| "OBS sent an unexpected Hello.".to_string())?;
        if hello.op != 0 {
            return Err("OBS did not send a Hello opcode.".into());
        }
        let auth = match hello.d.authentication {
            Some(ch) => {
                let pw = password.ok_or_else(|| {
                    "OBS WebSocket requires a password. Store it in Settings (keyring). The password is never written to the repo or settings.json.".to_string()
                })?;
                Some(auth_string(pw, &ch.salt, &ch.challenge))
            }
            None => None,
        };
        let identify = IdentifyMsg {
            op: 1,
            d: IdentifyData {
                rpc_version: hello.d.rpc_version.unwrap_or(1),
                authentication: auth,
            },
        };
        write_json(&mut socket, &identify)?;
        let identified = read_json(&mut socket)?;
        let op = identified.get("op").and_then(|v| v.as_u64()).unwrap_or(99);
        if op != 2 {
            let msg = identified
                .pointer("/d/comment")
                .or_else(|| identified.pointer("/d/message"))
                .and_then(|v| v.as_str())
                .unwrap_or("Identify failed");
            return Err(format!("OBS refused Identify: {msg}"));
        }
        let _ = hello.d.obs_websocket_version;
        Ok(Self { socket, next_id: 1 })
    }

    pub fn start_record(&mut self) -> Result<(), String> {
        self.request("StartRecord")
    }

    pub fn stop_record(&mut self) -> Result<(), String> {
        self.request("StopRecord")
    }

    pub fn start_stream(&mut self) -> Result<(), String> {
        self.request("StartStream")
    }

    pub fn stop_stream(&mut self) -> Result<(), String> {
        self.request("StopStream")
    }

    fn request(&mut self, kind: &str) -> Result<(), String> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = RequestMsg {
            op: 6,
            d: RequestData {
                request_type: kind.into(),
                request_id: id.to_string(),
            },
        };
        write_json(&mut self.socket, &msg)?;
        loop {
            let value = read_json(&mut self.socket)?;
            let op = value.get("op").and_then(|v| v.as_u64()).unwrap_or(0);
            if op != 7 {
                continue;
            }
            let ok = value
                .pointer("/d/requestStatus/result")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if ok {
                return Ok(());
            }
            let comment = value
                .pointer("/d/requestStatus/comment")
                .and_then(|v| v.as_str())
                .unwrap_or("OBS request failed");
            return Err(format!("{kind}: {comment}"));
        }
    }
}

fn write_json<T: Serialize>(
    socket: &mut tungstenite::WebSocket<TcpStream>,
    value: &T,
) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    socket
        .send(tungstenite::Message::Text(text.into()))
        .map_err(|e| format!("OBS WebSocket write failed: {e}"))
}

fn read_json(socket: &mut tungstenite::WebSocket<TcpStream>) -> Result<Value, String> {
    match socket.read() {
        Ok(tungstenite::Message::Text(text)) => {
            serde_json::from_str(&text).map_err(|e| format!("OBS sent invalid JSON: {e}"))
        }
        Ok(tungstenite::Message::Ping(p)) => {
            let _ = socket.send(tungstenite::Message::Pong(p));
            read_json(socket)
        }
        Ok(tungstenite::Message::Close(_)) => Err("OBS closed the WebSocket.".into()),
        Ok(_) => read_json(socket),
        Err(e) => Err(format!("OBS WebSocket read failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_auth_is_stable_sha256_base64() {
        let auth = auth_string(
            "supersecretpassword",
            "DZPMI91s71f9AWD6qFs7p4aDtQ7S2lRo",
            "ztX1sjy17fOFwcTEH8PSbwSy9H4YkNET",
        );
        assert_eq!(auth, auth_string(
            "supersecretpassword",
            "DZPMI91s71f9AWD6qFs7p4aDtQ7S2lRo",
            "ztX1sjy17fOFwcTEH8PSbwSy9H4YkNET",
        ));
        assert_ne!(
            auth,
            auth_string("other", "DZPMI91s71f9AWD6qFs7p4aDtQ7S2lRo", "ztX1sjy17fOFwcTEH8PSbwSy9H4YkNET")
        );
        assert_eq!(STANDARD.decode(&auth).unwrap().len(), 32);
    }
}
