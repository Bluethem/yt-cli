use crate::error::{Result, YtcliError};
use crate::state::{AppState, Track};

pub const PREV_RESTART_SECS: f64 = 3.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrevAction {
    SeekZero,
    GoTo(usize),
}

pub fn replace_queue(state: &mut AppState, track: Track) {
    state.queue = vec![track.clone()];
    state.current_index = Some(0);
    state.now_playing = Some(track);
}

pub fn enqueue(state: &mut AppState, track: Track) {
    state.queue.push(track);
}

pub fn clear_pending(state: &mut AppState) {
    let Some(index) = state.current_index else {
        state.queue.clear();
        return;
    };
    if index >= state.queue.len() {
        state.queue.clear();
        state.current_index = None;
        return;
    }
    let current = state.queue[index].clone();
    state.queue = vec![current.clone()];
    state.current_index = Some(0);
    state.now_playing = Some(current);
}

pub fn try_next_index(state: &AppState) -> Result<usize> {
    let index = state.current_index.ok_or(YtcliError::NoNext)?;
    let next = index + 1;
    if next >= state.queue.len() {
        return Err(YtcliError::NoNext);
    }
    Ok(next)
}

pub fn decide_prev(force: bool, time_pos: f64, current_index: usize) -> Result<PrevAction> {
    if force || time_pos <= PREV_RESTART_SECS {
        if current_index == 0 {
            return Err(YtcliError::NoPrevious);
        }
        return Ok(PrevAction::GoTo(current_index - 1));
    }
    Ok(PrevAction::SeekZero)
}

pub fn apply_index(state: &mut AppState, index: usize) {
    state.current_index = Some(index);
    state.now_playing = state.queue.get(index).cloned();
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn replace_queue_sets_single_track_and_index() {
        let mut state = AppState {
            queue: vec![track("old", "Old")],
            current_index: Some(0),
            ..Default::default()
        };

        let t = track("new", "New");
        replace_queue(&mut state, t.clone());

        assert_eq!(state.queue, vec![t.clone()]);
        assert_eq!(state.current_index, Some(0));
        assert_eq!(state.now_playing, Some(t));
    }

    #[test]
    fn enqueue_appends_without_changing_index_or_now_playing() {
        let playing = track("a", "A");
        let mut state = AppState {
            queue: vec![playing.clone()],
            current_index: Some(0),
            now_playing: Some(playing),
            ..Default::default()
        };

        let added = track("b", "B");
        enqueue(&mut state, added.clone());

        assert_eq!(state.queue.len(), 2);
        assert_eq!(state.queue[1], added);
        assert_eq!(state.current_index, Some(0));
        assert_eq!(state.now_playing.as_ref().unwrap().id, "a");
    }

    #[test]
    fn clear_pending_keeps_only_current_track() {
        let mut state = AppState {
            queue: vec![track("a", "A"), track("b", "B"), track("c", "C")],
            current_index: Some(1),
            now_playing: Some(track("b", "B")),
            ..Default::default()
        };

        clear_pending(&mut state);

        assert_eq!(state.queue.len(), 1);
        assert_eq!(state.queue[0].id, "b");
        assert_eq!(state.current_index, Some(0));
        assert_eq!(state.now_playing.as_ref().unwrap().id, "b");
    }

    #[test]
    fn clear_pending_without_current_empties_queue() {
        let mut state = AppState {
            queue: vec![track("a", "A"), track("b", "B")],
            current_index: None,
            ..Default::default()
        };

        clear_pending(&mut state);

        assert!(state.queue.is_empty());
        assert!(state.current_index.is_none());
    }

    #[test]
    fn try_next_index_returns_next_when_available() {
        let mut state = AppState {
            queue: vec![track("a", "A"), track("b", "B"), track("c", "C")],
            current_index: Some(0),
            ..Default::default()
        };

        assert_eq!(try_next_index(&state).unwrap(), 1);

        state.current_index = Some(1);
        assert_eq!(try_next_index(&state).unwrap(), 2);
    }

    #[test]
    fn try_next_index_errors_at_end_or_without_index() {
        let mut state = AppState {
            queue: vec![track("a", "A"), track("b", "B")],
            current_index: Some(1),
            ..Default::default()
        };

        assert!(matches!(try_next_index(&state), Err(YtcliError::NoNext)));

        state.current_index = None;
        assert!(matches!(try_next_index(&state), Err(YtcliError::NoNext)));
    }

    #[test]
    fn decide_prev_at_start_of_track_goes_to_previous() {
        assert_eq!(decide_prev(false, 0.0, 1).unwrap(), PrevAction::GoTo(0));
        assert_eq!(decide_prev(false, 2.9, 1).unwrap(), PrevAction::GoTo(0));
    }

    #[test]
    fn decide_prev_after_threshold_seeks_zero() {
        assert_eq!(decide_prev(false, 3.1, 1).unwrap(), PrevAction::SeekZero);
    }

    #[test]
    fn decide_prev_force_always_goes_to_previous() {
        assert_eq!(decide_prev(true, 3.1, 1).unwrap(), PrevAction::GoTo(0));
        assert_eq!(decide_prev(true, 0.0, 2).unwrap(), PrevAction::GoTo(1));
    }

    #[test]
    fn decide_prev_errors_when_no_previous() {
        assert!(matches!(
            decide_prev(false, 0.0, 0),
            Err(YtcliError::NoPrevious)
        ));
        assert!(matches!(
            decide_prev(true, 3.1, 0),
            Err(YtcliError::NoPrevious)
        ));
    }

    #[test]
    fn apply_index_sets_current_and_now_playing() {
        let mut state = AppState {
            queue: vec![track("a", "A"), track("b", "B")],
            ..Default::default()
        };

        apply_index(&mut state, 1);

        assert_eq!(state.current_index, Some(1));
        assert_eq!(state.now_playing.as_ref().unwrap().id, "b");
    }
}
