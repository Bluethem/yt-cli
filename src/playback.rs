use crate::cache::AudioCache;
use crate::state::Track;
use crate::youtube::YtDlp;
use crate::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackSource {
    Local(PathBuf),
    Stream(String),
}

/// Prefer valid local cache file; otherwise resolve stream URL.
pub fn resolve(track: &Track, cache: &AudioCache, yt_dlp: &YtDlp) -> Result<PlaybackSource> {
    if let Some((_entry, path)) = cache.get(&track.id)? {
        return Ok(PlaybackSource::Local(path));
    }
    let url = yt_dlp.resolve_audio_url(&track.webpage_url)?;
    Ok(PlaybackSource::Stream(url))
}

pub fn source_path_or_url(source: &PlaybackSource) -> String {
    match source {
        PlaybackSource::Local(path) => path.to_string_lossy().into_owned(),
        PlaybackSource::Stream(url) => url.clone(),
    }
}

pub fn is_local(source: &PlaybackSource) -> bool {
    matches!(source, PlaybackSource::Local(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::AudioCache;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolve_prefers_local_file() {
        let dir = tempdir().unwrap();
        let cache = AudioCache::at(dir.path().to_path_buf());
        let track = Track {
            id: "vid1".into(),
            title: "T".into(),
            uploader: "U".into(),
            webpage_url: "https://www.youtube.com/watch?v=vid1".into(),
            duration_secs: None,
        };
        fs::create_dir_all(cache.root()).unwrap();
        let opus = cache.file_path_for_id("vid1");
        fs::write(&opus, b"opus").unwrap();
        cache
            .put(&track, 4, "2026-01-01T00:00:00Z".into())
            .unwrap();

        // yt_dlp should not be called — use a dummy that would fail if invoked
        let yt = YtDlp {
            bin: "/nonexistent/yt-dlp-should-not-run".into(),
        };
        let source = resolve(&track, &cache, &yt).unwrap();
        assert_eq!(source, PlaybackSource::Local(opus));
    }
}
