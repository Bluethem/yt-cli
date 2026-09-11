use crate::cache::{AudioCache, DEFAULT_BITRATE_KBPS};
use crate::state::Track;
use crate::util::which_bin;
use crate::youtube::YtDlp;
use crate::{Result, YtcliError};
use std::fs;
use std::process::Command;

pub fn ensure_ffmpeg() -> Result<()> {
    which_bin("ffmpeg")
}

/// Download bestaudio via yt-dlp and encode to Opus 160 into the cache.
/// Skips if already cached. Returns true if a new file was written.
pub fn download_track(track: &Track, cache: &AudioCache, yt_dlp: &YtDlp) -> Result<bool> {
    if cache.has(&track.id)? {
        return Ok(false);
    }
    yt_dlp.ensure_bin()?;
    ensure_ffmpeg()?;

    fs::create_dir_all(cache.root()).map_err(|e| YtcliError::StateIo {
        path: cache.root().to_path_buf(),
        detail: e.to_string(),
    })?;

    let tmp_tpl = cache.root().join(format!("{}.download.XXXXXX", track.id));
    let tmp_out = cache
        .root()
        .join(format!("{}.ytdlp.%(ext)s", track.id));

    let status = Command::new(&yt_dlp.bin)
        .args([
            "-f",
            "bestaudio/bestaudio*",
            "--no-playlist",
            "-o",
            tmp_out.to_str().ok_or_else(|| {
                YtcliError::YtDlp("ruta temporal no UTF-8".into())
            })?,
            "--",
            &track.webpage_url,
        ])
        .status()
        .map_err(|e| YtcliError::YtDlp(e.to_string()))?;
    if !status.success() {
        cleanup_globs(cache, &track.id);
        return Err(YtcliError::YtDlp(
            "yt-dlp no pudo descargar el audio".into(),
        ));
    }

    let downloaded = find_download_file(cache, &track.id).ok_or_else(|| {
        cleanup_globs(cache, &track.id);
        YtcliError::YtDlp("no se encontró el archivo descargado por yt-dlp".into())
    })?;

    let final_path = cache.file_path_for_id(&track.id);
    // Extension must end in .opus so ffmpeg picks the muxer (not `.opus.tmp`).
    let tmp_opus = cache.root().join(format!("{}.tmp.opus", track.id));
    let bitrate = format!("{DEFAULT_BITRATE_KBPS}k");

    let ff = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            downloaded.to_str().unwrap_or(""),
            "-vn",
            "-c:a",
            "libopus",
            "-b:a",
            &bitrate,
            "-f",
            "opus",
            "-v",
            "error",
            tmp_opus.to_str().unwrap_or(""),
        ])
        .status()
        .map_err(|e| YtcliError::YtDlp(format!("ffmpeg: {e}")))?;
    let _ = fs::remove_file(&downloaded);
    if !ff.success() {
        let _ = fs::remove_file(&tmp_opus);
        cleanup_globs(cache, &track.id);
        return Err(YtcliError::YtDlp(
            "ffmpeg no pudo convertir a Opus".into(),
        ));
    }

    fs::rename(&tmp_opus, &final_path).map_err(|e| YtcliError::StateIo {
        path: final_path.clone(),
        detail: e.to_string(),
    })?;
    let bytes = fs::metadata(&final_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let _ = tmp_tpl;
    cache.put(
        track,
        bytes,
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )?;
    Ok(true)
}

fn find_download_file(cache: &AudioCache, id: &str) -> Option<std::path::PathBuf> {
    let prefix = format!("{id}.ytdlp.");
    let entries = fs::read_dir(cache.root()).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && !name.contains(".tmp.") {
            return Some(entry.path());
        }
    }
    None
}

fn cleanup_globs(cache: &AudioCache, id: &str) {
    if let Ok(entries) = fs::read_dir(cache.root()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&format!("{id}.ytdlp."))
                || name == format!("{id}.tmp.opus")
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::AudioCache;
    use tempfile::tempdir;

    #[test]
    fn skip_when_already_cached() {
        let dir = tempdir().unwrap();
        let cache = AudioCache::at(dir.path().to_path_buf());
        let track = Track {
            id: "cached1".into(),
            title: "T".into(),
            uploader: "U".into(),
            webpage_url: "https://y".into(),
            duration_secs: None,
        };
        fs::create_dir_all(cache.root()).unwrap();
        fs::write(cache.file_path_for_id("cached1"), b"x").unwrap();
        cache
            .put(&track, 1, "t".into())
            .unwrap();
        let yt = YtDlp {
            bin: "/should/not/run".into(),
        };
        assert!(!download_track(&track, &cache, &yt).unwrap());
    }
}
