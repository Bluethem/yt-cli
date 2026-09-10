use crate::state::Track;
use crate::util::which_bin;
use crate::{Result, YtcliError};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct YtDlp {
    pub bin: String,
}

impl Default for YtDlp {
    fn default() -> Self {
        Self {
            bin: "yt-dlp".into(),
        }
    }
}

impl YtDlp {
    pub fn ensure_bin(&self) -> Result<()> {
        which_bin(&self.bin)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        self.ensure_bin()?;
        let args = search_args(query, limit);
        let output = Command::new(&self.bin)
            .args(&args)
            .output()
            .map_err(|e| YtcliError::YtDlp(e.to_string()))?;
        if !output.status.success() {
            return Err(YtcliError::YtDlp(
                String::from_utf8_lossy(&output.stderr).into(),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tracks = parse_search_jsonl(&stdout)?;
        if tracks.is_empty() {
            return Err(YtcliError::NoSearchResults);
        }
        Ok(tracks)
    }

    pub fn resolve_audio_url(&self, webpage_url: &str) -> Result<String> {
        self.ensure_bin()?;
        let output = Command::new(&self.bin)
            .args([
                "-f",
                "bestaudio/bestaudio*",
                "-g",
                "--no-playlist",
                webpage_url,
            ])
            .output()
            .map_err(|e| YtcliError::YtDlp(e.to_string()))?;
        if !output.status.success() {
            return Err(YtcliError::YtDlp(
                String::from_utf8_lossy(&output.stderr).into(),
            ));
        }
        let url = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if url.is_empty() {
            return Err(YtcliError::YtDlp("URL de audio vacía".into()));
        }
        Ok(url)
    }

    /// Fetch tracks from a YouTube playlist URL (flat, no download).
    /// Returns `(playlist_title, tracks)`. Invalid entries are skipped.
    pub fn fetch_playlist(
        &self,
        playlist_url: &str,
        limit: usize,
    ) -> Result<(Option<String>, Vec<Track>)> {
        self.ensure_bin()?;
        let args = playlist_args(playlist_url, limit);
        let output = Command::new(&self.bin)
            .args(&args)
            .output()
            .map_err(|e| YtcliError::YtDlp(e.to_string()))?;
        if !output.status.success() {
            return Err(YtcliError::YtDlp(
                String::from_utf8_lossy(&output.stderr).into(),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (title, tracks, skipped) = parse_playlist_jsonl(&stdout);
        if tracks.is_empty() {
            return Err(YtcliError::NoSearchResults);
        }
        if skipped > 0 {
            eprintln!("Advertencia: se omitieron {skipped} entradas inválidas de la playlist.");
        }
        Ok((title, tracks))
    }
}

pub(crate) fn playlist_args(playlist_url: &str, limit: usize) -> Vec<String> {
    vec![
        "--flat-playlist".into(),
        "--dump-json".into(),
        "--no-download".into(),
        "--playlist-end".into(),
        limit.to_string(),
        playlist_url.into(),
    ]
}

/// Parse flat playlist JSONL. Skips bad lines; returns optional title from first row with playlist.
pub(crate) fn parse_playlist_jsonl(stdout: &str) -> (Option<String>, Vec<Track>, usize) {
    let mut tracks = Vec::new();
    let mut skipped = 0usize;
    let mut title = None;
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<YtRow>(line) else {
            skipped += 1;
            continue;
        };
        if title.is_none() {
            title = row.playlist_title.clone().or(row.playlist.clone());
        }
        match row.into_track() {
            Ok(track) => tracks.push(track),
            Err(_) => skipped += 1,
        }
    }
    (title, tracks, skipped)
}

pub(crate) fn search_args(query: &str, limit: usize) -> Vec<String> {
    vec![
        "--flat-playlist".into(),
        "--dump-json".into(),
        "--no-download".into(),
        format!("ytsearch{limit}:{query}"),
    ]
}

pub(crate) fn parse_search_jsonl(stdout: &str) -> Result<Vec<Track>> {
    let mut tracks = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: YtRow = serde_json::from_str(line).map_err(|e| YtcliError::Json(e.to_string()))?;
        tracks.push(row.into_track()?);
    }
    Ok(tracks)
}

#[derive(Debug, Deserialize)]
struct YtRow {
    id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    webpage_url: Option<String>,
    url: Option<String>,
    duration: Option<f64>,
    playlist: Option<String>,
    playlist_title: Option<String>,
}

impl YtRow {
    fn into_track(self) -> Result<Track> {
        let id = self.id.unwrap_or_default();
        let title = self.title.unwrap_or_else(|| "(sin título)".into());
        let uploader = self
            .uploader
            .or(self.channel)
            .unwrap_or_else(|| "(desconocido)".into());
        let webpage_url = self
            .webpage_url
            .or(self.url)
            .filter(|u| u.starts_with("http"))
            .or_else(|| {
                if id.is_empty() {
                    None
                } else {
                    Some(format!("https://www.youtube.com/watch?v={id}"))
                }
            })
            .ok_or_else(|| YtcliError::Json("entrada sin webpage_url".into()))?;
        Ok(Track {
            id,
            title,
            uploader,
            webpage_url,
            duration_secs: self.duration.map(|d| d as u64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jsonl_two_tracks() {
        let raw = include_str!("../tests/fixtures/yt_search_two.jsonl");
        let tracks = parse_search_jsonl(raw).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id, "vid1");
        assert_eq!(tracks[0].uploader, "Artist A");
        assert_eq!(tracks[1].uploader, "Artist B");
        assert_eq!(
            tracks[1].webpage_url,
            "https://www.youtube.com/watch?v=vid2"
        );
        assert_eq!(tracks[1].duration_secs, Some(90));
    }

    #[test]
    fn search_args_include_limit_and_query() {
        let args = search_args("lofi", 5);
        assert!(args
            .iter()
            .any(|a| a.contains("ytsearch5:lofi") || a.as_str() == "ytsearch5:lofi"));
    }

    #[test]
    fn playlist_args_include_url_and_limit() {
        let args = playlist_args("https://youtube.com/playlist?list=PLtest", 50);
        assert!(args.iter().any(|a| a == "--playlist-end"));
        assert!(args.iter().any(|a| a == "50"));
        assert!(args.iter().any(|a| a.contains("playlist?list=PLtest")));
    }

    #[test]
    fn parse_playlist_jsonl_skips_invalid_and_keeps_title() {
        let raw = include_str!("../tests/fixtures/yt_playlist_three.jsonl");
        let (title, tracks, skipped) = parse_playlist_jsonl(raw);
        assert_eq!(title.as_deref(), Some("My Mix"));
        assert_eq!(tracks.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!(tracks[0].id, "pl1");
        assert_eq!(tracks[1].uploader, "Artist B");
    }
}
