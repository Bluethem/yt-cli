use crate::cli::{Cli, Commands};
use crate::player::{self, Mpv};
use crate::queue::{self, PrevAction};
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
        Commands::Shuffle => cmd_shuffle(),
        Commands::Playlist { url, play, limit } => cmd_playlist(&url, play, limit),
        Commands::Loop => cmd_loop(true),
        Commands::Unloop => cmd_loop(false),
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

fn cmd_add(target: &str) -> Result<()> {
    let yt_dlp = YtDlp::default();
    let track = match target.parse::<usize>() {
        Ok(index) if index >= 1 => resolve_play_target(&AppState::load()?, target)?,
        _ => yt_dlp
            .search(target, 1)?
            .into_iter()
            .next()
            .ok_or(YtcliError::NoSearchResults)?,
    };
    let mut state = AppState::load()?;
    queue::enqueue(&mut state, track.clone());
    let position = state.queue.len();
    state.save()?;
    println!(
        "Añadida a la cola en la posición {position}: {} — {}",
        track.title, track.uploader
    );
    Ok(())
}

pub(crate) fn format_queue_line(
    index_1based: usize,
    track: &Track,
    is_current: bool,
    loop_on: bool,
) -> String {
    let marker = if is_current { ">" } else { " " };
    let loop_mark = if is_current && loop_on { " 🔁" } else { "" };
    format!(
        "{marker} {index_1based}. {} — {}{loop_mark}",
        track.title, track.uploader
    )
}

fn cmd_queue() -> Result<()> {
    let state = AppState::load()?;
    if state.queue.is_empty() {
        println!("La cola está vacía.");
        return Ok(());
    }

    for (index, track) in state.queue.iter().enumerate() {
        let is_current = state.current_index == Some(index);
        println!(
            "{}",
            format_queue_line(index + 1, track, is_current, state.loop_current)
        );
    }
    Ok(())
}

fn cmd_next() -> Result<()> {
    let mut state = AppState::load()?;
    let next = queue::try_next_index(&state)?;
    queue::clear_loop(&mut state);
    load_queue_index(&mut state, next)?;
    state.save()?;
    warn_if_watch_fails(&mut state);
    Ok(())
}

fn cmd_prev(force: bool) -> Result<()> {
    let mut state = AppState::load()?;
    let current = state.current_index.ok_or(YtcliError::NoPrevious)?;
    let socket = require_socket(&mut state)?;
    let time_pos = player::time_pos(&socket).unwrap_or(0.0);

    match queue::decide_prev(force, time_pos, current)? {
        PrevAction::SeekZero => {
            player::seek_absolute(&socket, 0.0)?;
            println!("⏮ Pista reiniciada");
        }
        PrevAction::GoTo(index) => {
            queue::clear_loop(&mut state);
            load_queue_index_at_socket(&mut state, index, &socket)?;
            state.save()?;
        }
    }
    warn_if_watch_fails(&mut state);
    Ok(())
}

fn cmd_clear() -> Result<()> {
    let mut state = AppState::load()?;
    let previous_len = state.queue.len();
    queue::clear_pending(&mut state);
    state.save()?;
    if state.queue.len() < previous_len {
        println!("Cola pendiente eliminada.");
    }
    Ok(())
}

fn cmd_shuffle() -> Result<()> {
    let mut state = AppState::load()?;
    let count = queue::shuffle_pending(&mut state);
    state.save()?;
    if count == 0 {
        println!("Nada que mezclar en la cola pendiente.");
    } else {
        println!("Mezcladas {count} pistas pendientes.");
    }
    Ok(())
}

fn cmd_playlist(url: &str, play: bool, limit: usize) -> Result<()> {
    let yt_dlp = YtDlp::default();
    yt_dlp.ensure_bin()?;

    let (title, tracks) = if crate::spotify::is_spotify_url(url) {
        crate::spotify::expand_to_tracks(url, limit)?
    } else {
        yt_dlp.fetch_playlist(url, limit)?
    };
    let label = title.unwrap_or_else(|| "playlist".into());

    if play {
        let mpv = Mpv::default();
        mpv.ensure_bin()?;
        let first = tracks[0].clone();
        let url_audio = yt_dlp.resolve_audio_url(&first.webpage_url)?;
        let mut state = AppState::load()?;
        queue::clear_loop(&mut state);
        state.queue = tracks;
        queue::apply_index(&mut state, 0);
        let socket = AppState::socket_path()?;
        let pid = mpv.ensure_running(&socket, state.mpv_pid)?;
        if pid != 0 {
            state.mpv_pid = Some(pid);
        }
        state.ipc_socket = Some(socket.clone());
        Mpv::loadfile(&socket, &url_audio)?;
        let n = state.queue.len();
        state.save()?;
        warn_if_watch_fails(&mut state);
        println!(
            "▶ Playlist «{label}»: {n} pistas. Reproduciendo {} — {}",
            first.title, first.uploader
        );
        return Ok(());
    }

    let added = tracks.len();
    let mut state = AppState::load()?;
    for track in tracks {
        queue::enqueue(&mut state, track);
    }
    state.save()?;
    println!("Encoladas {added} pistas desde «{label}».");
    Ok(())
}

fn cmd_loop(enable: bool) -> Result<()> {
    let mut state = AppState::load()?;
    if enable {
        if state.now_playing.is_none() && state.current_index.is_none() {
            return Err(YtcliError::NotPlaying);
        }
        state.loop_current = true;
        state.save()?;
        println!("🔁 Loop activado en la pista actual.");
    } else if !state.loop_current {
        println!("Loop ya estaba desactivado.");
    } else {
        queue::clear_loop(&mut state);
        state.save()?;
        println!("Loop desactivado.");
    }
    Ok(())
}

fn cmd_watch() -> Result<()> {
    crate::watch::run_watch_loop()
}

fn cmd_play(target: &str) -> Result<()> {
    let mpv = Mpv::default();
    let yt_dlp = YtDlp::default();
    mpv.ensure_bin()?;

    let mut state = AppState::load()?;
    let track = resolve_target(&state, &yt_dlp, target)?;
    queue::clear_loop(&mut state);
    queue::replace_queue(&mut state, track.clone());
    let url = yt_dlp.resolve_audio_url(&track.webpage_url)?;
    let socket = AppState::socket_path()?;
    let pid = mpv.ensure_running(&socket, state.mpv_pid)?;

    if pid != 0 {
        state.mpv_pid = Some(pid);
    }
    state.ipc_socket = Some(socket.clone());
    Mpv::loadfile(&socket, &url)?;
    state.save()?;
    warn_if_watch_fails(&mut state);
    println!("▶ {} — {}", track.title, track.uploader);
    Ok(())
}

fn warn_if_watch_fails(state: &mut AppState) {
    if let Err(error) = crate::watch::ensure_running(state) {
        eprintln!("Advertencia: no se pudo iniciar el auto-avance: {error}");
    }
}

fn resolve_target(state: &AppState, yt_dlp: &YtDlp, target: &str) -> Result<Track> {
    match target.parse::<usize>() {
        Ok(index) if index >= 1 => resolve_play_target(state, target),
        _ => yt_dlp
            .search(target, 1)?
            .into_iter()
            .next()
            .ok_or(YtcliError::NoSearchResults),
    }
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
    crate::watch::kill_watch(&mut state)?;
    let fallback = AppState::socket_path()?;
    if let Some(socket) = live_socket(state.ipc_socket.as_deref(), &fallback) {
        Mpv::quit(&socket)?;
    }
    state.clear_playback();
    state.clear_queue();
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
    let socket = match require_socket(&mut state) {
        Ok(socket) => socket,
        Err(YtcliError::NotPlaying) => {
            println!("No hay nada reproduciéndose.");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    print_status(&state, &socket)
}

fn cmd_now() -> Result<()> {
    let mut state = AppState::load()?;
    let socket = match require_socket(&mut state) {
        Ok(socket) => socket,
        Err(YtcliError::NotPlaying) => {
            println!("No hay nada reproduciéndose.");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    print_status(&state, &socket)
}

fn format_duration(duration_secs: Option<u64>) -> String {
    match duration_secs {
        Some(seconds) => format_clock(seconds as f64),
        None => "--:--".into(),
    }
}

fn load_queue_index(state: &mut AppState, index: usize) -> Result<()> {
    let socket = require_socket(state)?;
    load_queue_index_at_socket(state, index, &socket)
}

fn load_queue_index_at_socket(state: &mut AppState, index: usize, socket: &Path) -> Result<()> {
    let track = state.queue.get(index).cloned().ok_or(YtcliError::NoNext)?;
    let url = YtDlp::default().resolve_audio_url(&track.webpage_url)?;
    Mpv::loadfile(socket, &url)?;
    queue::apply_index(state, index);
    println!("▶ {} — {}", track.title, track.uploader);
    Ok(())
}

fn print_status(state: &AppState, socket: &Path) -> Result<()> {
    let track = state
        .current_index
        .and_then(|index| state.queue.get(index))
        .or(state.now_playing.as_ref());
    let Some(track) = track else {
        println!("mpv está activo, pero no hay información de la pista.");
        return Ok(());
    };
    let current = state.current_index.map_or(1, |index| index + 1);
    let total = state.queue.len().max(1);
    let position = player::time_pos(socket)
        .map(format_clock)
        .unwrap_or_else(|_| "?:??".into());
    let duration = player::duration(socket)
        .map(format_clock)
        .unwrap_or_else(|_| "?:??".into());
    println!(
        "{} [{current}/{total}] {} — {}  {position} / {duration}",
        if state.loop_current { "🔁" } else { "▶" },
        track.title,
        track.uploader
    );
    Ok(())
}

fn format_clock(secs: f64) -> String {
    let seconds = secs.max(0.0) as u64;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
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
    fn format_clock_formats_minutes_and_seconds() {
        assert_eq!(format_clock(65.0), "1:05");
    }

    #[test]
    fn format_clock_includes_hours_when_needed() {
        assert_eq!(format_clock(3661.0), "1:01:01");
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
    fn format_queue_line_marks_loop_on_current() {
        let track = Track {
            id: "a".into(),
            title: "T".into(),
            uploader: "U".into(),
            webpage_url: "https://y".into(),
            duration_secs: None,
        };
        let line = format_queue_line(1, &track, true, true);
        assert!(line.contains('>'));
        assert!(line.contains('🔁'));
        assert!(line.contains("T — U"));

        let line2 = format_queue_line(2, &track, false, true);
        assert!(line2.starts_with(' ') || line2.starts_with("  "));
        assert!(!line2.contains('🔁')); // loop solo en la actual
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
