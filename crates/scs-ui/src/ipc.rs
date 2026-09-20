use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join("shadow-creator-studio.sock")
}

pub fn send_action(action: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path())
        .map_err(|_| "The app is not running. Open Shadow Creator Studio first, then use the shortcut.".to_string())?;
    stream
        .write_all(action.as_bytes())
        .map_err(|e| format!("Could not send the shortcut: {e}"))?;
    Ok(())
}

pub fn listen() -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("scs-ipc".into())
        .spawn(move || {
            let path = socket_path();
            let _ = std::fs::remove_file(&path);
            let Ok(listener) = UnixListener::bind(&path) else {
                return;
            };
            for incoming in listener.incoming() {
                let Ok(mut stream) = incoming else { continue };
                let mut buf = String::new();
                if stream.read_to_string(&mut buf).is_ok() {
                    let action = buf.trim().to_string();
                    if !action.is_empty() && tx.send(action).is_err() {
                        break;
                    }
                }
            }
        })
        .ok();
    rx
}

pub fn parse_cli_action(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--action" {
            return args.get(i + 1).cloned();
        }
        if let Some(rest) = args[i].strip_prefix("--action=") {
            return Some(rest.to_string());
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_action_flag() {
        let args = vec!["app".into(), "--action".into(), "start-stop".into()];
        assert_eq!(parse_cli_action(&args).as_deref(), Some("start-stop"));
        assert_eq!(
            parse_cli_action(&["--action=marker".into()]).as_deref(),
            Some("marker")
        );
    }
}
