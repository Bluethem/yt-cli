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

pub fn get_property_f64(socket: &Path, name: &str) -> Result<f64> {
    let response = send_ipc_json(
        socket,
        &json!({ "command": ["get_property", name] }),
    )?;
    data_as_f64(socket, &response)
}

pub fn time_pos(socket: &Path) -> Result<f64> {
    get_property_f64(socket, "time-pos")
}

pub fn duration(socket: &Path) -> Result<f64> {
    get_property_f64(socket, "duration")
}

pub fn seek_absolute(socket: &Path, seconds: f64) -> Result<()> {
    send_ipc(
        socket,
        &json!({ "command": ["seek", seconds, "absolute"] }),
    )
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
    let response = write_ipc_and_read(socket, payload)?;
    ipc_response_result(socket, &response)
}

fn send_ipc_json(socket: &Path, payload: &Value) -> Result<Value> {
    let response = write_ipc_and_read(socket, payload)?;
    ipc_response_value(socket, &response)
}

fn write_ipc_and_read(socket: &Path, payload: &Value) -> Result<String> {
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
    Ok(response)
}

fn ipc_response_result(socket: &Path, response: &str) -> Result<()> {
    if let Ok(value) = serde_json::from_str::<Value>(response.trim()) {
        check_ipc_error(socket, &value)?;
    }
    Ok(())
}

fn ipc_response_value(socket: &Path, response: &str) -> Result<Value> {
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return Err(YtcliError::MpvIpc {
            path: socket.to_path_buf(),
            detail: "respuesta IPC vacía".into(),
        });
    }

    let value: Value = serde_json::from_str(trimmed).map_err(|error| YtcliError::MpvIpc {
        path: socket.to_path_buf(),
        detail: format!("respuesta IPC inválida: {error}"),
    })?;
    check_ipc_error(socket, &value)?;
    Ok(value)
}

fn check_ipc_error(socket: &Path, value: &Value) -> Result<()> {
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
    Ok(())
}

fn data_as_f64(socket: &Path, value: &Value) -> Result<f64> {
    let data = value.get("data").ok_or_else(|| YtcliError::MpvIpc {
        path: socket.to_path_buf(),
        detail: "respuesta IPC sin campo data".into(),
    })?;
    if data.is_null() {
        return Err(YtcliError::MpvIpc {
            path: socket.to_path_buf(),
            detail: "propiedad sin valor".into(),
        });
    }
    data.as_f64()
        .or_else(|| data.as_i64().map(|n| n as f64))
        .ok_or_else(|| YtcliError::MpvIpc {
            path: socket.to_path_buf(),
            detail: format!("data no numérica: {data}"),
        })
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

    #[test]
    fn get_property_response_parses_f64_data() {
        let socket = Path::new("/tmp/mpv.sock");
        let response = ipc_response_value(
            socket,
            r#"{"data":42.5,"error":"success","request_id":0}"#,
        )
        .unwrap();

        assert_eq!(data_as_f64(socket, &response).unwrap(), 42.5);
    }

    #[test]
    fn get_property_response_parses_integer_data_as_f64() {
        let socket = Path::new("/tmp/mpv.sock");
        let response =
            ipc_response_value(socket, r#"{"data":180,"error":null,"request_id":1}"#).unwrap();

        assert_eq!(data_as_f64(socket, &response).unwrap(), 180.0);
    }

    #[test]
    fn get_property_time_pos_fixture() {
        let socket = Path::new("/tmp/mpv.sock");
        let response = ipc_response_value(
            socket,
            r#"{"data":125.432,"error":"success","request_id":2}"#,
        )
        .unwrap();

        assert_eq!(data_as_f64(socket, &response).unwrap(), 125.432);
    }

    #[test]
    fn get_property_duration_fixture() {
        let socket = Path::new("/tmp/mpv.sock");
        let response = ipc_response_value(
            socket,
            r#"{"data":3661.0,"error":"success","request_id":3}"#,
        )
        .unwrap();

        assert_eq!(data_as_f64(socket, &response).unwrap(), 3661.0);
    }

    #[test]
    fn get_property_error_response_is_ipc_error() {
        let socket = Path::new("/tmp/mpv.sock");
        let error = ipc_response_value(socket, r#"{"error":"property unavailable"}"#).unwrap_err();

        assert!(matches!(
            error,
            YtcliError::MpvIpc { path, detail }
                if path == socket && detail == "property unavailable"
        ));
    }

    #[test]
    fn get_property_missing_data_is_ipc_error() {
        let socket = Path::new("/tmp/mpv.sock");
        let response = ipc_response_value(socket, r#"{"error":"success"}"#).unwrap();
        let error = data_as_f64(socket, &response).unwrap_err();

        assert!(matches!(
            error,
            YtcliError::MpvIpc { path, detail }
                if path == socket && detail == "respuesta IPC sin campo data"
        ));
    }

    #[test]
    fn get_property_null_data_is_ipc_error() {
        let socket = Path::new("/tmp/mpv.sock");
        let response = ipc_response_value(socket, r#"{"data":null,"error":"success"}"#).unwrap();
        let error = data_as_f64(socket, &response).unwrap_err();

        assert!(matches!(
            error,
            YtcliError::MpvIpc { path, detail }
                if path == socket && detail == "propiedad sin valor"
        ));
    }

    #[test]
    fn get_property_non_numeric_data_is_ipc_error() {
        let socket = Path::new("/tmp/mpv.sock");
        let response =
            ipc_response_value(socket, r#"{"data":"n/a","error":"success"}"#).unwrap();
        let error = data_as_f64(socket, &response).unwrap_err();

        assert!(matches!(
            error,
            YtcliError::MpvIpc { path, detail }
                if path == socket && detail.starts_with("data no numérica:")
        ));
    }

    #[test]
    fn seek_absolute_payload_uses_absolute_mode() {
        let payload = ipc_command_json(&["seek", "42.5", "absolute"]);

        assert!(payload.ends_with('\n'));
        let value: Value = serde_json::from_str(payload.trim_end()).unwrap();
        assert_eq!(value, json!({ "command": ["seek", "42.5", "absolute"] }));
    }

    #[test]
    fn empty_or_invalid_ipc_response_is_error_for_json_path() {
        let socket = Path::new("/tmp/mpv.sock");

        assert!(ipc_response_value(socket, "").is_err());
        assert!(ipc_response_value(socket, "not json").is_err());
    }
}
