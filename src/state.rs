use crate::{Result, YtcliError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub webpage_url: String,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppState {
    pub mpv_pid: Option<u32>,
    pub ipc_socket: Option<PathBuf>,
    pub now_playing: Option<Track>,
    #[serde(default)]
    pub last_search: Vec<Track>,
    #[serde(default)]
    pub queue: Vec<Track>,
    #[serde(default)]
    pub current_index: Option<usize>,
    #[serde(default)]
    pub watch_pid: Option<u32>,
    #[serde(default)]
    pub loop_current: bool,
}

impl AppState {
    pub fn cache_dir() -> Result<PathBuf> {
        let base = dirs::cache_dir().ok_or_else(|| YtcliError::StateIo {
            path: PathBuf::from("(cache_dir)"),
            detail: "dirs::cache_dir() devolvió None".into(),
        })?;
        Ok(base.join("ytcli"))
    }

    pub fn state_path() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("state.json"))
    }

    pub fn socket_path() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("mpv.sock"))
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(path).map_err(|e| YtcliError::StateIo {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })?;
        match serde_json::from_str(&data) {
            Ok(state) => Ok(state),
            Err(error) => {
                eprintln!(
                    "Advertencia: no se pudo leer el estado en {}: {error}. Se usará un estado vacío.",
                    path.display()
                );
                Ok(Self::default())
            }
        }
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&Self::state_path()?)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| YtcliError::StateIo {
                path: parent.to_path_buf(),
                detail: e.to_string(),
            })?;
        }
        let data =
            serde_json::to_string_pretty(self).map_err(|e| YtcliError::Json(e.to_string()))?;
        let mut tmp_path = path.as_os_str().to_owned();
        tmp_path.push(".tmp");
        let tmp_path = PathBuf::from(tmp_path);
        fs::write(&tmp_path, data).map_err(|e| YtcliError::StateIo {
            path: tmp_path.clone(),
            detail: e.to_string(),
        })?;
        fs::rename(&tmp_path, path).map_err(|e| YtcliError::StateIo {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::state_path()?)
    }

    pub fn clear_playback(&mut self) {
        self.mpv_pid = None;
        self.ipc_socket = None;
        self.now_playing = None;
        self.watch_pid = None;
        self.loop_current = false;
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.current_index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        let mut s = AppState::default();
        s.last_search.push(Track {
            id: "abc".into(),
            title: "T".into(),
            uploader: "U".into(),
            webpage_url: "https://youtube.com/watch?v=abc".into(),
            duration_secs: Some(10),
        });
        s.save_to(&path).unwrap();
        let loaded = AppState::load_from(&path).unwrap();
        assert_eq!(loaded.last_search.len(), 1);
        assert_eq!(loaded.last_search[0].id, "abc");
    }

    #[test]
    fn save_replaces_state_via_temporary_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        let tmp_path = dir.path().join("state.json.tmp");
        fs::write(&tmp_path, "contenido parcial").unwrap();

        AppState::default().save_to(&path).unwrap();

        assert!(path.exists());
        assert!(!tmp_path.exists());
        assert_eq!(AppState::load_from(&path).unwrap(), AppState::default());
    }

    fn track(id: &str, title: &str) -> Track {
        Track {
            id: id.into(),
            title: title.into(),
            uploader: "u".into(),
            webpage_url: format!("http://{id}"),
            duration_secs: None,
        }
    }

    #[test]
    fn load_legacy_state_without_queue_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, r#"{"last_search":[]}"#).unwrap();
        let s = AppState::load_from(&path).unwrap();
        assert!(s.queue.is_empty());
        assert!(s.current_index.is_none());
        assert!(s.watch_pid.is_none());
    }

    #[test]
    fn clear_playback_keeps_search_and_queue() {
        let mut s = AppState {
            mpv_pid: Some(1),
            ipc_socket: Some(PathBuf::from("/tmp/x.sock")),
            now_playing: Some(track("x", "t")),
            watch_pid: Some(99),
            last_search: vec![track("x", "t")],
            queue: vec![track("a", "A")],
            current_index: Some(0),
            loop_current: true,
        };
        s.clear_playback();
        assert!(s.mpv_pid.is_none());
        assert!(s.now_playing.is_none());
        assert!(s.watch_pid.is_none());
        assert!(!s.loop_current);
        assert_eq!(s.last_search.len(), 1);
        assert_eq!(s.queue.len(), 1);
        assert_eq!(s.current_index, Some(0));
    }

    #[test]
    fn load_legacy_state_defaults_loop_current_false() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, r#"{"last_search":[],"queue":[]}"#).unwrap();
        let s = AppState::load_from(&path).unwrap();
        assert!(!s.loop_current);
    }

    #[test]
    fn clear_playback_keeps_queue() {
        let mut s = AppState::default();
        s.queue.push(track("a", "A"));
        s.current_index = Some(0);
        s.mpv_pid = Some(1);
        s.clear_playback();
        assert!(s.mpv_pid.is_none());
        assert_eq!(s.queue.len(), 1);
        assert_eq!(s.current_index, Some(0));
    }

    #[test]
    fn clear_queue_empties_queue_and_index() {
        let mut s = AppState::default();
        s.queue.push(track("a", "A"));
        s.current_index = Some(0);
        s.mpv_pid = Some(1);
        s.clear_queue();
        assert!(s.queue.is_empty());
        assert!(s.current_index.is_none());
        assert_eq!(s.mpv_pid, Some(1));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let s = AppState::load_from(&path).unwrap();
        assert!(s.last_search.is_empty());
    }

    #[test]
    fn load_corrupt_json_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, "{not valid json").unwrap();

        let state = AppState::load_from(&path).unwrap();

        assert_eq!(state, AppState::default());
    }
}
