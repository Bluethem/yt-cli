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
        fs::write(path, data).map_err(|e| YtcliError::StateIo {
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
    fn clear_playback_keeps_search() {
        let mut s = AppState {
            mpv_pid: Some(1),
            ipc_socket: Some(PathBuf::from("/tmp/x.sock")),
            now_playing: Some(Track {
                id: "x".into(),
                title: "t".into(),
                uploader: "u".into(),
                webpage_url: "http://x".into(),
                duration_secs: None,
            }),
            last_search: vec![Track {
                id: "x".into(),
                title: "t".into(),
                uploader: "u".into(),
                webpage_url: "http://x".into(),
                duration_secs: None,
            }],
        };
        s.clear_playback();
        assert!(s.mpv_pid.is_none());
        assert!(s.now_playing.is_none());
        assert_eq!(s.last_search.len(), 1);
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
