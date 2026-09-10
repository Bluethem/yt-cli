use crate::cli::{Cli, Commands};
use crate::player::Mpv;
use crate::state::{AppState, Track};
use crate::youtube::YtDlp;
use crate::{Result, YtcliError};
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Search { query, limit } => cmd_search(&query, limit),
        Commands::Play { target } => cmd_play(&target),
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

fn cmd_play(target: &str) -> Result<()> {
    let mut state = AppState::load()?;
    let track = match target.parse::<usize>() {
        Ok(index) if index >= 1 => resolve_play_target(&state, target)?,
        _ => YtDlp::default()
            .search(target, 1)?
            .into_iter()
            .next()
            .ok_or(YtcliError::NoSearchResults)?,
    };

    let yt_dlp = YtDlp::default();
    let url = yt_dlp.resolve_audio_url(&track.webpage_url)?;
    let socket = AppState::socket_path()?;
    let mpv = Mpv::default();
    let pid = mpv.ensure_running(&socket, state.mpv_pid)?;
    Mpv::loadfile(&socket, &url)?;

    if pid != 0 {
        state.mpv_pid = Some(pid);
    }
    state.ipc_socket = Some(socket);
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
    let socket = state.ipc_socket.clone();
    if let Some(socket) = socket {
        if Mpv::is_alive(&socket) {
            return Ok(socket);
        }
    }

    state.clear_playback();
    state.save()?;
    Err(YtcliError::NotPlaying)
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
}
