use crate::state::Track;
use crate::{Result, YtcliError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_NAME: &str = "liked";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalPlaylist {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub source: Option<String>,
    pub tracks: Vec<Track>,
}

pub struct PlaylistStore {
    root: PathBuf,
}

impl PlaylistStore {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn system() -> Result<Self> {
        let base = dirs::data_dir().ok_or_else(|| YtcliError::StateIo {
            path: PathBuf::from("(data_dir)"),
            detail: "dirs::data_dir() devolvió None".into(),
        })?;
        Ok(Self::at(base.join("ytcli").join("playlists")))
    }

    pub fn append_track(&self, name: &str, track: Track) -> Result<LocalPlaylist> {
        self.append_tracks(name, vec![track])
    }

    pub fn append_tracks(&self, name: &str, tracks: Vec<Track>) -> Result<LocalPlaylist> {
        let display_name = name.trim().to_string();
        validate_name(&display_name)?;
        let mut pl = match self.load(&display_name) {
            Ok(existing) => existing,
            Err(YtcliError::PlaylistNotFound(_)) => {
                let now = now_rfc3339();
                LocalPlaylist {
                    name: display_name.clone(),
                    created_at: now.clone(),
                    updated_at: now,
                    source: None,
                    tracks: Vec::new(),
                }
            }
            Err(e) => return Err(e),
        };

        for track in tracks {
            if pl.tracks.iter().any(|t| t.id == track.id) {
                continue;
            }
            pl.tracks.push(track);
        }
        pl.updated_at = now_rfc3339();
        self.save(&pl)?;
        Ok(pl)
    }

    pub fn replace_tracks(
        &self,
        name: &str,
        tracks: Vec<Track>,
        source: Option<String>,
    ) -> Result<LocalPlaylist> {
        let display_name = name.trim().to_string();
        validate_name(&display_name)?;
        let now = now_rfc3339();
        let created_at = match self.load(&display_name) {
            Ok(existing) => existing.created_at,
            Err(YtcliError::PlaylistNotFound(_)) => now.clone(),
            Err(e) => return Err(e),
        };
        let pl = LocalPlaylist {
            name: display_name,
            created_at,
            updated_at: now,
            source,
            tracks,
        };
        self.save(&pl)?;
        Ok(pl)
    }

    pub fn load(&self, name: &str) -> Result<LocalPlaylist> {
        let path = self.path_for(name)?;
        if !path.exists() {
            return Err(YtcliError::PlaylistNotFound(name.trim().to_string()));
        }
        let data = fs::read_to_string(&path).map_err(|e| YtcliError::StateIo {
            path: path.clone(),
            detail: e.to_string(),
        })?;
        serde_json::from_str(&data).map_err(|e| YtcliError::Json(e.to_string()))
    }

    pub fn list(&self) -> Result<Vec<(String, usize)>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| YtcliError::StateIo {
            path: self.root.clone(),
            detail: e.to_string(),
        })? {
            let entry = entry.map_err(|e| YtcliError::StateIo {
                path: self.root.clone(),
                detail: e.to_string(),
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let data = fs::read_to_string(&path).map_err(|e| YtcliError::StateIo {
                path: path.clone(),
                detail: e.to_string(),
            })?;
            let pl: LocalPlaylist =
                serde_json::from_str(&data).map_err(|e| YtcliError::Json(e.to_string()))?;
            out.push((pl.name, pl.tracks.len()));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.path_for(name)?;
        if !path.exists() {
            return Err(YtcliError::PlaylistNotFound(name.trim().to_string()));
        }
        fs::remove_file(&path).map_err(|e| YtcliError::StateIo {
            path,
            detail: e.to_string(),
        })
    }

    fn path_for(&self, name: &str) -> Result<PathBuf> {
        let slug = slugify(name)?;
        Ok(self.root.join(format!("{slug}.json")))
    }

    fn save(&self, pl: &LocalPlaylist) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| YtcliError::StateIo {
            path: self.root.clone(),
            detail: e.to_string(),
        })?;
        let path = self.path_for(&pl.name)?;
        save_atomic(&path, pl)
    }
}

pub fn validate_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(YtcliError::InvalidPlaylistName(String::new()));
    }
    if name == "." || name == ".." {
        return Err(YtcliError::InvalidPlaylistName(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(YtcliError::InvalidPlaylistName(name.to_string()));
    }
    for c in name.chars() {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c.is_whitespace()) {
            return Err(YtcliError::InvalidPlaylistName(name.to_string()));
        }
    }
    Ok(())
}

pub fn slugify(name: &str) -> Result<String> {
    validate_name(name)?;
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .filter(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        .collect();
    if slug.is_empty() {
        return Err(YtcliError::InvalidPlaylistName(name.trim().to_string()));
    }
    Ok(slug)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn save_atomic(path: &Path, pl: &LocalPlaylist) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| YtcliError::StateIo {
            path: parent.to_path_buf(),
            detail: e.to_string(),
        })?;
    }
    let data = serde_json::to_string_pretty(pl).map_err(|e| YtcliError::Json(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_path_traversal_names() {
        assert!(validate_name("..").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("").is_err());
        assert!(validate_name("ok-list").is_ok());
    }

    #[test]
    fn append_dedup_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = PlaylistStore::at(dir.path().to_path_buf());
        let t1 = Track {
            id: "1".into(),
            title: "A".into(),
            uploader: "U".into(),
            webpage_url: "u".into(),
            duration_secs: None,
        };
        let t1b = Track {
            id: "1".into(),
            title: "A2".into(),
            uploader: "U".into(),
            webpage_url: "u".into(),
            duration_secs: None,
        };
        let t2 = Track {
            id: "2".into(),
            title: "B".into(),
            uploader: "U".into(),
            webpage_url: "u".into(),
            duration_secs: None,
        };
        store.append_track("liked", t1).unwrap();
        store.append_track("liked", t1b).unwrap();
        let pl = store.append_track("liked", t2).unwrap();
        assert_eq!(pl.tracks.len(), 2);
        assert_eq!(pl.tracks[0].title, "A"); // no reemplaza título en dedup
    }

    #[test]
    fn replace_sets_source_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let store = PlaylistStore::at(dir.path().to_path_buf());
        let t1 = Track {
            id: "1".into(),
            title: "A".into(),
            uploader: "U".into(),
            webpage_url: "u".into(),
            duration_secs: None,
        };
        store.append_track("rock", t1.clone()).unwrap();
        let t2 = Track {
            id: "9".into(),
            title: "Z".into(),
            uploader: "U".into(),
            webpage_url: "u".into(),
            duration_secs: None,
        };
        let pl = store
            .replace_tracks(
                "rock",
                vec![t2],
                Some("https://youtube.com/playlist?list=X".into()),
            )
            .unwrap();
        assert_eq!(pl.tracks.len(), 1);
        assert_eq!(pl.tracks[0].id, "9");
        assert_eq!(
            pl.source.as_deref(),
            Some("https://youtube.com/playlist?list=X")
        );
    }

    #[test]
    fn delete_missing_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let store = PlaylistStore::at(dir.path().to_path_buf());
        let err = store.delete("nope").unwrap_err();
        assert!(matches!(err, YtcliError::PlaylistNotFound(_)));
    }
}
