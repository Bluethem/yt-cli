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

/// Shuffle only tracks after the current one. Returns how many pending tracks were shuffled.
pub fn shuffle_pending(state: &mut AppState) -> usize {
    let Some(index) = state.current_index else {
        if state.queue.len() < 2 {
            return 0;
        }
        // No current playback: shuffle entire queue in place.
        let mut rng = SimpleRng::from_entropy();
        fisher_yates(&mut state.queue, &mut rng);
        return state.queue.len();
    };
    if index + 1 >= state.queue.len() {
        return 0;
    }
    let pending = &mut state.queue[index + 1..];
    let count = pending.len();
    let mut rng = SimpleRng::from_entropy();
    fisher_yates(pending, &mut rng);
    count
}

pub fn clear_loop(state: &mut AppState) {
    state.loop_current = false;
}

/// Tiny LCG so we avoid a rand dependency for shuffle.
struct SimpleRng(u64);

impl SimpleRng {
    fn from_entropy() -> Self {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5A17);
        seed ^= std::process::id() as u64;
        Self(seed | 1)
    }

    #[cfg(test)]
    fn from_seed(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes LCG
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }

    fn gen_range(&mut self, len: usize) -> usize {
        (self.next_u64() as usize) % len
    }
}

fn fisher_yates<T>(items: &mut [T], rng: &mut SimpleRng) {
    for i in (1..items.len()).rev() {
        let j = rng.gen_range(i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
pub fn shuffle_pending_with_seed(state: &mut AppState, seed: u64) -> usize {
    let Some(index) = state.current_index else {
        if state.queue.len() < 2 {
            return 0;
        }
        let mut rng = SimpleRng::from_seed(seed);
        fisher_yates(&mut state.queue, &mut rng);
        return state.queue.len();
    };
    if index + 1 >= state.queue.len() {
        return 0;
    }
    let pending = &mut state.queue[index + 1..];
    let count = pending.len();
    let mut rng = SimpleRng::from_seed(seed);
    fisher_yates(pending, &mut rng);
    count
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

    #[test]
    fn shuffle_pending_keeps_current_and_preserves_pending_set() {
        let mut state = AppState {
            queue: vec![
                track("a", "A"),
                track("b", "B"),
                track("c", "C"),
                track("d", "D"),
            ],
            current_index: Some(0),
            now_playing: Some(track("a", "A")),
            ..Default::default()
        };

        let count = shuffle_pending_with_seed(&mut state, 42);
        assert_eq!(count, 3);
        assert_eq!(state.queue[0].id, "a");
        assert_eq!(state.current_index, Some(0));
        let mut pending: Vec<_> = state.queue[1..].iter().map(|t| t.id.as_str()).collect();
        pending.sort();
        assert_eq!(pending, vec!["b", "c", "d"]);
    }

    #[test]
    fn fisher_yates_with_seed_can_reorder() {
        let mut items = vec!["b", "c", "d", "e", "f"];
        let original = items.clone();
        let mut rng = SimpleRng::from_seed(7);
        fisher_yates(&mut items, &mut rng);
        assert_ne!(items, original);
        items.sort();
        let mut expected = original;
        expected.sort();
        assert_eq!(items, expected);
    }

    #[test]
    fn shuffle_pending_noop_without_pending() {
        let mut state = AppState {
            queue: vec![track("a", "A")],
            current_index: Some(0),
            ..Default::default()
        };
        assert_eq!(shuffle_pending(&mut state), 0);
        assert_eq!(state.queue[0].id, "a");
    }

    #[test]
    fn clear_loop_sets_flag_false() {
        let mut state = AppState {
            loop_current: true,
            ..Default::default()
        };
        clear_loop(&mut state);
        assert!(!state.loop_current);
    }
}
