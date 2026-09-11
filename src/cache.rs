use crate::state::Track;
use crate::{Result, YtcliError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_BITRATE_KBPS: u32 = 160;
pub const DEFAULT_CODEC: &str = "opus";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    pub path: String,
    pub title: String,
    pub uploader: String,
    pub bitrate_kbps: u32,
    pub codec: String,
    pub bytes: u64,
    pub downloaded_at: String,
}

pub struct AudioCache {
    root: PathBuf,
}

impl AudioCache {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn system() -> Result<Self> {
        Ok(Self::at(crate::state::AppState::cache_dir()?.join("audio")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    pub fn file_path_for_id(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.opus"))
    }

    pub fn absolute_path(&self, entry: &CacheEntry) -> PathBuf {
        self.root.join(&entry.path)
    }

    fn load_index(&self) -> Result<HashMap<String, CacheEntry>> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let data = fs::read_to_string(&path).map_err(|e| YtcliError::StateIo {
            path: path.clone(),
            detail: e.to_string(),
        })?;
        serde_json::from_str(&data).map_err(|e| YtcliError::Json(format!("{}: {e}", path.display())))
    }

    fn save_index(&self, index: &HashMap<String, CacheEntry>) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| YtcliError::StateIo {
            path: self.root.clone(),
            detail: e.to_string(),
        })?;
        let data =
            serde_json::to_string_pretty(index).map_err(|e| YtcliError::Json(e.to_string()))?;
        let tmp = self.root.join("index.json.tmp");
        fs::write(&tmp, data).map_err(|e| YtcliError::StateIo {
            path: tmp.clone(),
            detail: e.to_string(),
        })?;
        fs::rename(&tmp, self.index_path()).map_err(|e| YtcliError::StateIo {
            path: self.index_path(),
            detail: e.to_string(),
        })
    }

    /// True if index has id and the opus file exists and is non-empty.
    pub fn has(&self, id: &str) -> Result<bool> {
        Ok(self.get(id)?.is_some())
    }

    pub fn get(&self, id: &str) -> Result<Option<(CacheEntry, PathBuf)>> {
        let index = self.load_index()?;
        let Some(entry) = index.get(id).cloned() else {
            return Ok(None);
        };
        let path = self.absolute_path(&entry);
        match fs::metadata(&path) {
            Ok(meta) if meta.is_file() && meta.len() > 0 => Ok(Some((entry, path))),
            _ => Ok(None),
        }
    }

    pub fn put(&self, track: &Track, file_bytes: u64, downloaded_at: String) -> Result<CacheEntry> {
        fs::create_dir_all(&self.root).map_err(|e| YtcliError::StateIo {
            path: self.root.clone(),
            detail: e.to_string(),
        })?;
        let entry = CacheEntry {
            path: format!("{}.opus", track.id),
            title: track.title.clone(),
            uploader: track.uploader.clone(),
            bitrate_kbps: DEFAULT_BITRATE_KBPS,
            codec: DEFAULT_CODEC.into(),
            bytes: file_bytes,
            downloaded_at,
        };
        let mut index = self.load_index()?;
        index.insert(track.id.clone(), entry.clone());
        self.save_index(&index)?;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: "T".into(),
            uploader: "U".into(),
            webpage_url: "https://y".into(),
            duration_secs: None,
        }
    }

    #[test]
    fn put_get_has_roundtrip() {
        let dir = tempdir().unwrap();
        let cache = AudioCache::at(dir.path().to_path_buf());
        let track = sample_track("abc123");
        let opus = cache.file_path_for_id("abc123");
        fs::create_dir_all(cache.root()).unwrap();
        fs::write(&opus, b"fake-opus-bytes").unwrap();
        cache
            .put(&track, 15, "2026-01-01T00:00:00Z".into())
            .unwrap();
        assert!(cache.has("abc123").unwrap());
        let (entry, path) = cache.get("abc123").unwrap().unwrap();
        assert_eq!(entry.bitrate_kbps, 160);
        assert_eq!(path, opus);
    }

    #[test]
    fn has_false_when_file_missing_despite_index() {
        let dir = tempdir().unwrap();
        let cache = AudioCache::at(dir.path().to_path_buf());
        let track = sample_track("gone");
        // put writes index but we don't create the opus file
        fs::create_dir_all(cache.root()).unwrap();
        let mut index = HashMap::new();
        index.insert(
            "gone".into(),
            CacheEntry {
                path: "gone.opus".into(),
                title: "T".into(),
                uploader: "U".into(),
                bitrate_kbps: 160,
                codec: "opus".into(),
                bytes: 1,
                downloaded_at: "t".into(),
            },
        );
        cache.save_index(&index).unwrap();
        let _ = track;
        assert!(!cache.has("gone").unwrap());
    }
}
