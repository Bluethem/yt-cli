use crate::player::{self, Mpv};
use crate::queue;
use crate::state::{AppState, Track};
use crate::youtube::YtDlp;
use crate::Result;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceOutcome {
    Advanced,
    Finished,
}

fn advance_after_eof_with<L, Q>(
    state: &mut AppState,
    mut load: L,
    mut quit: Q,
) -> Result<AdvanceOutcome>
where
    L: FnMut(&Track) -> Result<()>,
    Q: FnMut() -> Result<()>,
{
    match queue::try_next_index(state) {
        Ok(index) => {
            let track = state.queue[index].clone();
            load(&track)?;
            queue::apply_index(state, index);
            Ok(AdvanceOutcome::Advanced)
        }
        Err(crate::YtcliError::NoNext) => {
            quit()?;
            state.clear_playback();
            state.clear_queue();
            Ok(AdvanceOutcome::Finished)
        }
        Err(error) => Err(error),
    }
}

pub fn advance_after_eof(state: &mut AppState) -> Result<AdvanceOutcome> {
    let socket = state.ipc_socket.clone().unwrap_or(AppState::socket_path()?);
    let yt_dlp = YtDlp::default();
    advance_after_eof_with(
        state,
        |track| {
            let url = yt_dlp.resolve_audio_url(&track.webpage_url)?;
            Mpv::loadfile(&socket, &url)
        },
        || Mpv::quit(&socket),
    )
}

pub fn ensure_running(state: &mut AppState) -> Result<()> {
    if let Some(pid) = state.watch_pid {
        if process_is_watch(pid) {
            return Ok(());
        }
        state.watch_pid = None;
        state.save()?;
    }

    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "Advertencia: no se pudo localizar ytcli para iniciar el auto-avance: {error}"
            );
            return Ok(());
        }
    };
    let child = match Command::new(executable)
        .arg("watch")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("Advertencia: no se pudo iniciar el auto-avance: {error}");
            return Ok(());
        }
    };

    state.watch_pid = Some(child.id());
    state.save()
}

pub fn kill_watch(state: &mut AppState) -> Result<()> {
    if let Some(pid) = state.watch_pid.take() {
        if process_is_watch(pid) {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    }
    Ok(())
}

pub fn run_watch_loop() -> Result<()> {
    let own_pid = std::process::id();
    let mut initial_state = AppState::load()?;
    if initial_state
        .watch_pid
        .is_some_and(|pid| pid != own_pid && process_is_watch(pid))
    {
        return Ok(());
    }
    initial_state.watch_pid = Some(own_pid);
    initial_state.save()?;

    let mut had_playback = false;
    let mut observed_track = None;

    loop {
        let mut state = AppState::load()?;
        let track_identity = (
            state.current_index,
            state
                .current_index
                .and_then(|index| state.queue.get(index))
                .map(|track| track.id.clone()),
        );
        if observed_track.as_ref() != Some(&track_identity) {
            observed_track = Some(track_identity);
            had_playback = false;
        }
        let socket = state.ipc_socket.clone().unwrap_or(AppState::socket_path()?);

        match player::time_pos(&socket) {
            Ok(position) if position > 0.0 => had_playback = true,
            Ok(_) => {}
            Err(_) if !Mpv::is_alive(&socket) => {
                clear_own_pid(&mut state)?;
                return Ok(());
            }
            Err(_) => {}
        }

        match player::get_property_bool(&socket, "idle-active") {
            Ok(idle_or_eof) if should_trigger_advance(had_playback, idle_or_eof) => {
                let outcome = match advance_after_eof(&mut state) {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        clear_own_pid(&mut state)?;
                        return Err(error);
                    }
                };
                state.save()?;
                match outcome {
                    AdvanceOutcome::Advanced => {
                        observed_track = None;
                        had_playback = false;
                    }
                    AdvanceOutcome::Finished => return Ok(()),
                }
            }
            Ok(_) => {}
            Err(_) if !Mpv::is_alive(&socket) => {
                clear_own_pid(&mut state)?;
                return Ok(());
            }
            Err(_) => {}
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn should_trigger_advance(had_playback: bool, idle_or_eof: bool) -> bool {
    had_playback && idle_or_eof
}

fn process_is_watch(pid: u32) -> bool {
    let proc_dir = Path::new("/proc").join(pid.to_string());
    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };
    let Ok(process_exe) = fs::read_link(proc_dir.join("exe")) else {
        return false;
    };
    let Ok(cmdline) = fs::read(proc_dir.join("cmdline")) else {
        return false;
    };

    matches_watch_process(&current_exe, &process_exe, &cmdline)
}

fn matches_watch_process(current_exe: &Path, process_exe: &Path, cmdline: &[u8]) -> bool {
    if current_exe != process_exe {
        return false;
    }

    let args: Vec<&[u8]> = cmdline
        .split(|byte| *byte == 0)
        .filter(|argument| !argument.is_empty())
        .collect();

    if args.get(1).copied() != Some(b"watch") {
        return false;
    }

    args.first().is_none_or(|argv0| argv0_looks_like_binary(argv0, current_exe))
}

fn argv0_looks_like_binary(argv0: &[u8], current_exe: &Path) -> bool {
    let Ok(argv0_str) = std::str::from_utf8(argv0) else {
        return false;
    };
    let argv0_path = Path::new(argv0_str);

    if argv0_path == current_exe {
        return true;
    }

    match (argv0_path.file_name(), current_exe.file_name()) {
        (Some(argv0_name), Some(exe_name)) if argv0_name == exe_name => return true,
        _ => {}
    }

    match (argv0_path.canonicalize(), current_exe.canonicalize()) {
        (Ok(resolved_argv0), Ok(resolved_exe)) => resolved_argv0 == resolved_exe,
        _ => false,
    }
}

fn clear_own_pid(state: &mut AppState) -> Result<()> {
    let own_pid = std::process::id();
    if state.watch_pid == Some(own_pid) {
        state.watch_pid = None;
        state.save()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Track;
    use std::process::{Command, Stdio};

    fn track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: id.into(),
            uploader: "u".into(),
            webpage_url: format!("https://example.com/{id}"),
            duration_secs: None,
        }
    }

    #[test]
    fn advance_after_eof_loads_and_selects_next_track() {
        let mut state = AppState {
            queue: vec![track("a"), track("b")],
            current_index: Some(0),
            now_playing: Some(track("a")),
            ..Default::default()
        };
        let mut loaded = None;

        let outcome = advance_after_eof_with(
            &mut state,
            |next| {
                loaded = Some(next.id.clone());
                Ok(())
            },
            || Ok(()),
        )
        .unwrap();

        assert_eq!(outcome, AdvanceOutcome::Advanced);
        assert_eq!(loaded.as_deref(), Some("b"));
        assert_eq!(state.current_index, Some(1));
        assert_eq!(
            state.now_playing.as_ref().map(|track| track.id.as_str()),
            Some("b")
        );
    }

    #[test]
    fn advance_after_eof_stops_and_clears_state_at_end_of_queue() {
        let mut state = AppState {
            mpv_pid: Some(10),
            ipc_socket: Some("/tmp/mpv.sock".into()),
            watch_pid: Some(11),
            queue: vec![track("a")],
            current_index: Some(0),
            now_playing: Some(track("a")),
            ..Default::default()
        };
        let mut quit_called = false;

        let outcome = advance_after_eof_with(
            &mut state,
            |_| panic!("no debe cargar otra pista"),
            || {
                quit_called = true;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(outcome, AdvanceOutcome::Finished);
        assert!(quit_called);
        assert!(state.queue.is_empty());
        assert!(state.current_index.is_none());
        assert!(state.now_playing.is_none());
        assert!(state.mpv_pid.is_none());
        assert!(state.ipc_socket.is_none());
        assert!(state.watch_pid.is_none());
    }

    #[test]
    fn watch_process_identity_requires_matching_executable_and_watch_argument() {
        let executable = Path::new("/usr/bin/ytcli");

        assert!(matches_watch_process(
            executable,
            executable,
            b"/usr/bin/ytcli\0watch\0"
        ));
        assert!(!matches_watch_process(
            executable,
            Path::new("/usr/bin/other"),
            b"/usr/bin/ytcli\0watch\0"
        ));
        assert!(!matches_watch_process(
            executable,
            executable,
            b"/usr/bin/ytcli\0status\0"
        ));
        assert!(!matches_watch_process(
            executable,
            executable,
            b"/usr/bin/ytcli\0play\0watch\0"
        ));
        assert!(!matches_watch_process(
            executable,
            executable,
            b"/usr/bin/other\0watch\0"
        ));
    }

    #[test]
    fn kill_watch_clears_mismatched_pid_without_killing_process() {
        let mut child = Command::new("sleep")
            .arg("10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut state = AppState {
            watch_pid: Some(child.id()),
            ..Default::default()
        };

        kill_watch(&mut state).unwrap();

        let still_running = child.try_wait().unwrap().is_none();
        if still_running {
            child.kill().unwrap();
            child.wait().unwrap();
        }
        assert!(still_running);
        assert!(state.watch_pid.is_none());
    }

    #[test]
    fn trigger_advance_requires_prior_playback_and_idle_or_eof() {
        assert!(should_trigger_advance(true, true));
        assert!(!should_trigger_advance(false, true));
        assert!(!should_trigger_advance(true, false));
        assert!(!should_trigger_advance(false, false));
    }
}
