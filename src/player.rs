use crate::util::which_bin;
use crate::{Result, YtcliError};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const IPC_TIMEOUT: Duration = Duration::from_secs(2);
const START_RETRIES: usize = 20;
const START_RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct Mpv {
    pub bin: String,
}

impl Default for Mpv {
    fn default() -> Self {
        Self { bin: "mpv".into() }
    }
}

impl Mpv {
    pub fn ensure_bin(&self) -> Result<()> {
        which_bin(&self.bin)
    }

    pub fn is_alive(socket: &Path) -> bool {
        send_ipc(socket, &json!({ "command": ["get_property", "pause"] })).is_ok()
    }

    pub fn ensure_running(&self, socket: &Path, existing_pid: Option<u32>) -> Result<u32> {
        self.ensure_bin()?;
        if Self::is_alive(socket) {
            return Ok(existing_pid.unwrap_or(0));
        }

        if let Some(parent) = socket.parent() {
            fs::create_dir_all(parent).map_err(|error| ipc_error(socket, error))?;
        }
        match fs::remove_file(socket) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ipc_error(socket, error)),
        }

        let child = Command::new(&self.bin)
            .args([
                "--no-video",
                "--idle=yes",
                "--really-quiet",
                "--no-terminal",
                &format!("--input-ipc-server={}", socket.display()),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| ipc_error(socket, error))?;
        let pid = child.id();

        for _ in 0..START_RETRIES {
            if Self::is_alive(socket) {
                return Ok(pid);
            }
            thread::sleep(START_RETRY_DELAY);
        }

        Err(YtcliError::MpvIpc {
            path: socket.to_path_buf(),
            detail: "timeout esperando IPC".into(),
        })
    }

    pub fn loadfile(socket: &Path, url: &str) -> Result<()> {
        send_ipc(socket, &json!({ "command": ["loadfile", url, "replace"] }))
    }

    pub fn set_pause(socket: &Path, paused: bool) -> Result<()> {
        send_ipc(
            socket,
            &json!({ "command": ["set_property", "pause", paused] }),
        )
    }

    pub fn set_volume(socket: &Path, vol: u8) -> Result<()> {
        if vol > 100 {
            return Err(YtcliError::MpvIpc {
                path: socket.to_path_buf(),
                detail: format!("volumen fuera de rango: {vol} (esperado 0..=100)"),
            });
        }
        send_ipc(
            socket,
            &json!({ "command": ["set_property", "volume", vol] }),
        )
    }

    pub fn quit(socket: &Path) -> Result<()> {
        let _ = send_ipc(socket, &json!({ "command": ["quit"] }));
        Ok(())
    }
}

pub fn ipc_command_json(command: &[&str]) -> String {
    json_line(&json!({ "command": command }))
}

pub fn ipc_set_pause_json(paused: bool) -> String {
    json_line(&json!({ "command": ["set_property", "pause", paused] }))
}

pub fn ipc_loadfile_json(url: &str) -> String {
    json_line(&json!({ "command": ["loadfile", url, "replace"] }))
}

fn json_line(payload: &Value) -> String {
    let mut line = payload.to_string();
    line.push('\n');
    line
}

fn send_ipc(socket: &Path, payload: &Value) -> Result<()> {
    let mut stream = UnixStream::connect(socket).map_err(|error| ipc_error(socket, error))?;
    stream
        .set_read_timeout(Some(IPC_TIMEOUT))
        .map_err(|error| ipc_error(socket, error))?;
    stream
        .set_write_timeout(Some(IPC_TIMEOUT))
        .map_err(|error| ipc_error(socket, error))?;
    stream
        .write_all(json_line(payload).as_bytes())
        .map_err(|error| ipc_error(socket, error))?;

    let mut response = String::new();
    let _ = BufReader::new(stream).read_line(&mut response);
    ipc_response_result(socket, &response)
}

fn ipc_response_result(socket: &Path, response: &str) -> Result<()> {
    if let Ok(value) = serde_json::from_str::<Value>(&response) {
        if let Some(error) = value.get("error") {
            let succeeded = error.is_null() || error.as_str() == Some("success");
            if !succeeded {
                let detail = error
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| error.to_string());
                return Err(YtcliError::MpvIpc {
                    path: socket.to_path_buf(),
                    detail,
                });
            }
        }
    }
    Ok(())
}

fn ipc_error(socket: &Path, error: impl std::fmt::Display) -> YtcliError {
    YtcliError::MpvIpc {
        path: socket.to_path_buf(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn ipc_payload_set_pause_uses_typed_boolean_and_newline() {
        let payload = ipc_set_pause_json(true);

        assert!(payload.ends_with('\n'));
        let value: Value = serde_json::from_str(payload.trim_end()).unwrap();
        assert_eq!(value, json!({ "command": ["set_property", "pause", true] }));
    }

    #[test]
    fn ipc_payload_loadfile_uses_replace_mode_and_newline() {
        let payload = ipc_loadfile_json("https://example.com/a.m4a");

        assert!(payload.ends_with('\n'));
        let value: Value = serde_json::from_str(payload.trim_end()).unwrap();
        assert_eq!(
            value,
            json!({ "command": ["loadfile", "https://example.com/a.m4a", "replace"] })
        );
    }

    #[test]
    fn mpv_failure_response_is_an_ipc_error() {
        let socket = Path::new("/tmp/mpv.sock");
        let error = ipc_response_result(socket, r#"{"error":"property unavailable"}"#).unwrap_err();

        assert!(matches!(
            error,
            YtcliError::MpvIpc { path, detail }
                if path == socket && detail == "property unavailable"
        ));
    }

    #[test]
    fn mpv_success_null_and_unparseable_responses_are_best_effort() {
        let socket = Path::new("/tmp/mpv.sock");

        assert!(ipc_response_result(socket, r#"{"error":"success"}"#).is_ok());
        assert!(ipc_response_result(socket, r#"{"error":null}"#).is_ok());
        assert!(ipc_response_result(socket, "").is_ok());
        assert!(ipc_response_result(socket, "not json").is_ok());
    }
}
