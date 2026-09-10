use crate::cli::{Cli, Commands};
use crate::player::Mpv;
use crate::state::{AppState, Track};
use crate::youtube::YtDlp;
use crate::{Result, YtcliError};
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Search { query, limit } => cmd_search(&query, limit),
        Commands::Play { target } => cmd_play(&target),
        Commands::Add { target } => cmd_add(&target),
        Commands::Queue => cmd_queue(),
        Commands::Next => cmd_next(),
        Commands::Prev { force } => cmd_prev(force),
        Commands::Clear => cmd_clear(),
        Commands::Watch => cmd_watch(),
        Commands::Pause => cmd_pause(true),
        Commands::Resume => cmd_pause(false),
        Commands::Stop => cmd_stop(),
        Commands::Volume { level } => cmd_volume(level),
        Commands::Status => cmd_status(),
        Commands::Now => cmd_now(),
    }
}

fn cmd_search(query: &str, limit: usize) -> Result<()> {
    let tracks = YtDlp::default().search(query, limit)?;
    let mut state = AppState::load()?;
    state.last_search = tracks;
    state.save()?;

    for (index, track) in state.last_search.iter().enumerate() {
        println!(
            "{}. {} — {} [{}]",
            index + 1,
            track.title,
            track.uploader,
            format_duration(track.duration_secs)
        );
    }
    Ok(())
}

fn cmd_add(_target: &str) -> Result<()> {
    Ok(())
}

fn cmd_queue() -> Result<()> {
    Ok(())
}

fn cmd_next() -> Result<()> {
    Ok(())
}

fn cmd_prev(_force: bool) -> Result<()> {
    Ok(())
}

fn cmd_clear() -> Result<()> {
    Ok(())
}

fn cmd_watch() -> Result<()> {
    Ok(())
}

fn cmd_play(target: &str) -> Result<()> {
    let mpv = Mpv::default();
    let yt_dlp = YtDlp::default();
    mpv.ensure_bin()?;
    yt_dlp.ensure_bin()?;

    let mut state = AppState::load()?;
    let track = match target.parse::<usize>() {
        Ok(index) if index >= 1 => resolve_play_target(&state, target)?,
        _ => yt_dlp
            .search(target, 1)?
            .into_iter()
            .next()
            .ok_or(YtcliError::NoSearchResults)?,
    };

    let url = yt_dlp.resolve_audio_url(&track.webpage_url)?;
    let socket = AppState::socket_path()?;
    let pid = mpv.ensure_running(&socket, state.mpv_pid)?;

    if pid != 0 {
        state.mpv_pid = Some(pid);
    }
    state.ipc_socket = Some(socket.clone());
    state.save()?;

    Mpv::loadfile(&socket, &url)?;
    state.now_playing = Some(track.clone());
    state.save()?;
    println!("▶ {} — {}", track.title, track.uploader);
    Ok(())
}

pub(crate) fn resolve_play_target(state: &AppState, target: &str) -> Result<Track> {
    let index = match target.parse::<usize>() {
        Ok(index) if index >= 1 => index,
        _ => return Err(YtcliError::NoSearchResults),
    };

    if state.last_search.is_empty() {
        return Err(YtcliError::NoLastSearch);
    }

    state
        .last_search
        .get(index - 1)
        .cloned()
        .ok_or(YtcliError::IndexOutOfRange {
            index,
            len: state.last_search.len(),
        })
}

fn require_socket(state: &mut AppState) -> Result<PathBuf> {
    let fallback = AppState::socket_path()?;
    if let Some(socket) = live_socket(state.ipc_socket.as_deref(), &fallback) {
        if state.ipc_socket.as_deref() != Some(socket.as_path()) {
            state.ipc_socket = Some(socket.clone());
            state.save()?;
        }
        return Ok(socket);
    }

    state.clear_playback();
    state.save()?;
    Err(YtcliError::NotPlaying)
}

fn live_socket(saved: Option<&Path>, fallback: &Path) -> Option<PathBuf> {
    if let Some(socket) = saved {
        if Mpv::is_alive(socket) {
            return Some(socket.to_path_buf());
        }
    }
    if saved != Some(fallback) && Mpv::is_alive(fallback) {
        return Some(fallback.to_path_buf());
    }
    None
}

fn cmd_pause(paused: bool) -> Result<()> {
    let mut state = AppState::load()?;
    let socket = require_socket(&mut state)?;
    Mpv::set_pause(&socket, paused)?;
    println!(
        "{}",
        if paused {
            "⏸ En pausa"
        } else {
            "▶ Reanudado"
        }
    );
    Ok(())
}

fn cmd_stop() -> Result<()> {
    let mut state = AppState::load()?;
    let socket = require_socket(&mut state)?;
    Mpv::quit(&socket)?;
    state.clear_playback();
    state.save()?;
    println!("⏹ Reproducción detenida");
    Ok(())
}

fn cmd_volume(level: u8) -> Result<()> {
    if level > 100 {
        return Err(YtcliError::InvalidVolume(level));
    }

    let mut state = AppState::load()?;
    let socket = require_socket(&mut state)?;
    Mpv::set_volume(&socket, level)?;
    println!("🔊 Volumen: {level}%");
    Ok(())
}

fn cmd_status() -> Result<()> {
    let mut state = AppState::load()?;
    match require_socket(&mut state) {
        Ok(_) => {}
        Err(YtcliError::NotPlaying) => {
            println!("No hay nada reproduciéndose.");
            return Ok(());
        }
        Err(error) => return Err(error),
    }

    if let Some(track) = state.now_playing {
        println!("Reproduciendo: {} — {}", track.title, track.uploader);
    } else {
        println!("mpv está activo.");
    }
    Ok(())
}

fn cmd_now() -> Result<()> {
    let mut state = AppState::load()?;
    match require_socket(&mut state) {
        Ok(_) => {}
        Err(YtcliError::NotPlaying) => {
            println!("No hay nada reproduciéndose.");
            return Ok(());
        }
        Err(error) => return Err(error),
    }

    if let Some(track) = state.now_playing {
        println!("▶ {} — {}", track.title, track.uploader);
    } else {
        println!("mpv está activo, pero no hay información de la pista.");
    }
    Ok(())
}

fn format_duration(duration_secs: Option<u64>) -> String {
    match duration_secs {
        Some(seconds) => format!("{}:{:02}", seconds / 60, seconds % 60),
        None => "--:--".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, Track};
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use tempfile::tempdir;

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
    fn play_target_index() {
        let mut state = AppState::default();
        state.last_search.push(track("a", "A"));
        state.last_search.push(track("b", "B"));

        let resolved = resolve_play_target(&state, "2").unwrap();

        assert_eq!(resolved.id, "b");
    }

    #[test]
    fn play_target_bad_index_without_previous_search() {
        let state = AppState::default();

        assert!(matches!(
            resolve_play_target(&state, "1"),
            Err(crate::YtcliError::NoLastSearch)
        ));
    }

    #[test]
    fn play_target_index_out_of_range() {
        let mut state = AppState::default();
        state.last_search.push(track("a", "A"));

        assert!(matches!(
            resolve_play_target(&state, "2"),
            Err(crate::YtcliError::IndexOutOfRange { index: 2, len: 1 })
        ));
    }

    #[test]
    fn volume_above_one_hundred_is_rejected_before_loading_state() {
        assert!(matches!(
            cmd_volume(101),
            Err(YtcliError::InvalidVolume(101))
        ));
    }

    #[test]
    fn live_socket_falls_back_when_saved_socket_is_dead() {
        let dir = tempdir().unwrap();
        let dead_socket = dir.path().join("dead.sock");
        let fallback = dir.path().join("mpv.sock");
        let listener = UnixListener::bind(&fallback).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            stream.write_all(b"{\"error\":\"success\"}\n").unwrap();
        });

        let selected = live_socket(Some(&dead_socket), &fallback);

        assert_eq!(selected.as_deref(), Some(fallback.as_path()));
        server.join().unwrap();
    }
}
