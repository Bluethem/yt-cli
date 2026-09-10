# ytcli Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Añadir cola de reproducción, progreso temporal en status/now, next/prev/prev -f, clear, y auto-avance vía watch IPC.

**Architecture:** Cola en `state.json` (`queue`, `current_index`, `watch_pid`). URLs con yt-dlp al vuelo. mpv sigue siendo un solo proceso; `ytcli watch` (hijo detach) escucha `end-file` y avanza o hace stop limpio al final.

**Tech Stack:** Rust existente (clap, serde, mpv IPC Unix); sin deps nuevas salvo que haga falta algo mínimo para señales/kill de watch.

**Spec:** `docs/superpowers/specs/2026-09-10-ytcli-queue-design.md`

## Global Constraints

- `play` reemplaza cola completa y reproduce; `add` encola sin saltar
- Auto-next al `end-file`; al final de cola → stop limpio (sin wrap/loop)
- `prev`: si time-pos > 3.0 → seek 0; else pista anterior; `prev -f` siempre anterior
- `status`/`now`: `▶ [i/n] Title — Artist  m:ss / m:ss` (horas si ≥ 3600)
- `clear`: cola = solo actual; no detiene reproducción
- `stop`: vacía cola + mata watch + quit mpv
- Índices UI 1-based; `current_index` 0-based
- Target play/add: igual que MVP (usize ≥ 1 → last_search; else query)
- No TUI, no shuffle, no sesión tipo Cursor CLI
- Mensajes de usuario en español
- Commits solo si el humano lo pide en la sesión SDD (o si eligen subagent-driven con commits explícitos)
- Trabajar en worktree/feature branch, no pisar main sin consentimiento

---

## File structure

| Path | Responsibility |
|------|----------------|
| `src/state.rs` | + `queue`, `current_index`, `watch_pid`; `clear_playback` también limpia cola/index/watch; helpers sync now_playing |
| `src/queue.rs` | Lógica pura: replace/add/clear_pending/next_index/prev_decision |
| `src/player.rs` | + `get_f64_property`, `time_pos`, `duration`, `seek`, `observe` helpers / read IPC response data |
| `src/watch.rs` | ensure/kill watch process; `run_watch_loop` for `ytcli watch` |
| `src/cli.rs` | Add/Queue/List/Next/Prev{force}/Clear/Watch |
| `src/commands.rs` | Wire all handlers; status con tiempo; stop mata watch |
| `src/error.rs` | `NoNext`, `NoPrevious`, etc. |
| `src/lib.rs` | mod queue, watch |
| `README.md` | Documentar nuevos comandos |

---

### Task 1: State + errores de cola

**Files:**
- Modify: `src/state.rs`, `src/error.rs`
- Test: `src/state.rs` tests

**Interfaces:**
- Produces: `AppState { queue: Vec<Track>, current_index: Option<usize>, watch_pid: Option<u32>, ... }` with `#[serde(default)]` on new fields for old state.json
- `clear_playback` clears mpv fields **and** `watch_pid`, `now_playing`; also clears `queue` + `current_index` (stop semantics). Add `clear_playback_keep_queue` only if needed — prefer: `stop` calls clear queue separately; `clear_playback` used when mpv dies may keep queue — **Spec stop:** vacía cola. When IPC dies mid-play: keep queue in state for recovery OR clear — choose: **on NotPlaying cleanup from dead IPC, do not wipe queue** (user may still `next`). Only `cmd_stop` / auto end-of-queue wipe queue.
- Actually simplify:
  - `clear_playback(&mut self)` — mpv_pid, ipc_socket, now_playing, watch_pid (not queue)
  - `clear_queue(&mut self)` — queue.clear(), current_index=None
  - `stop` calls both
  - Auto end-of-queue: both
- Errors: `NoNext`, `NoPrevious`

- [ ] **Step 1: Write failing tests** for serde default of new fields + clear helpers

```rust
#[test]
fn load_legacy_state_without_queue_fields() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state.json");
    fs::write(&path, r#"{"last_search":[]}"#).unwrap();
    let s = AppState::load_from(&path).unwrap();
    assert!(s.queue.is_empty());
    assert!(s.current_index.is_none());
    assert!(s.watch_pid.is_none());
}

#[test]
fn clear_playback_keeps_queue() {
    let mut s = AppState::default();
    s.queue.push(track("a","A"));
    s.current_index = Some(0);
    s.mpv_pid = Some(1);
    s.clear_playback();
    assert!(s.mpv_pid.is_none());
    assert_eq!(s.queue.len(), 1);
}
```

- [ ] **Step 2: Run tests — expect FAIL**
- [ ] **Step 3: Implement fields + errors + helpers**
- [ ] **Step 4: `cargo test state:: error::` PASS**
- [ ] **Step 5: Commit if session allows**

---

### Task 2: Módulo `queue` (lógica pura)

**Files:**
- Create: `src/queue.rs`
- Modify: `src/lib.rs`

**Interfaces:**
```rust
pub const PREV_RESTART_SECS: f64 = 3.0;

pub enum PrevAction { SeekZero, GoTo(usize) }

pub fn replace_queue(state: &mut AppState, track: Track);
pub fn enqueue(state: &mut AppState, track: Track);
pub fn clear_pending(state: &mut AppState); // queue=[current], index=0
pub fn try_next_index(state: &AppState) -> Result<usize>; // Err(NoNext)
pub fn decide_prev(force: bool, time_pos: f64, current_index: usize) -> Result<PrevAction>;
pub fn apply_index(state: &mut AppState, index: usize); // sets current_index + now_playing from queue
```

- [ ] **Step 1: Failing unit tests** covering replace, enqueue, clear_pending, try_next, decide_prev (time 0, 2.9, 3.1, force)
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Implement**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit if allowed**

---

### Task 3: Player — time-pos, duration, seek

**Files:**
- Modify: `src/player.rs`

**Interfaces:**
```rust
pub fn get_property_f64(socket: &Path, name: &str) -> Result<f64>;
pub fn time_pos(socket: &Path) -> Result<f64>; // get_property time-pos
pub fn duration(socket: &Path) -> Result<f64>;
pub fn seek_absolute(socket: &Path, seconds: f64) -> Result<()>; // ["seek", seconds, "absolute"]
```

Extend `send_ipc` (or add `send_ipc_json`) to return parsed `data` on success for get_property.

- [ ] **Step 1: Tests** for parsing get_property response JSON fixtures (unit, no mpv)
- [ ] **Step 2: Implement**
- [ ] **Step 3: `cargo test player::` PASS**

---

### Task 4: CLI surface

**Files:**
- Modify: `src/cli.rs`, stub arms in `src/commands.rs`

**Commands to add:**
```rust
Add { target: String },
Queue,  // alias List via #[command(visible_alias = "list")] on Queue OR both variants calling same
Next,
Prev { #[arg(short = 'f', long = "force")] force: bool },
Clear,
/// Interno / watch loop (documentar en README como avanzado)
Watch,
```

- [ ] **Step 1: Parse tests** for `add`, `prev -f`, `queue`, `list` if alias
- [ ] **Step 2: Implement clap + match stubs**
- [ ] **Step 3: PASS**

---

### Task 5: Wire play/add/next/prev/clear/status/stop

**Files:**
- Modify: `src/commands.rs`
- Optionally thin helpers in `src/queue.rs` already used

**Behavior:**
- `cmd_play`: resolve track → `queue::replace_queue` → load URL → save → `watch::ensure` (Task 6 may stub ensure as Ok/warn)
- `cmd_add`: resolve → `enqueue` → save → print posición en cola
- `cmd_next`: `try_next_index` → apply → resolve URL → loadfile → save
- `cmd_prev(force)`: get time_pos (0.0 if fail) → `decide_prev` → seek or load previous
- `cmd_clear`: `clear_pending` → save → print
- `cmd_stop`: kill watch (Task 6) → quit mpv → `clear_playback` + `clear_queue` → save
- `cmd_status`/`cmd_now`: print `▶ [{i}/{n}] {title} — {uploader}  {pos} / {dur}` using `format_clock(secs: f64)` supporting hours

Extract `resolve_target(state, yt, target) -> Result<Track>` shared by play/add.

Until Task 6: `watch::ensure` can be `Ok(())` stub with TODO, or no-op function in commands behind `#[cfg]` — prefer empty `watch.rs` with `ensure_watch` returning Ok and eprintln stub only if not implemented — **Task 5 calls `crate::watch::ensure_running(...)` that Task 6 fills; for Task 5 add stub `pub fn ensure_running(...) -> Result<()> { Ok(()) }`**.

- [ ] **Step 1: Unit tests** for `format_clock` (65 → 1:05, 3661 → 1:01:01)
- [ ] **Step 2: Implement wiring**
- [ ] **Step 3: `cargo test` PASS**
- [ ] **Step 4: Commit if allowed**

---

### Task 6: Watch process

**Files:**
- Create/fill: `src/watch.rs`
- Modify: `src/commands.rs` (`Watch` arm, stop/kill), `src/main` already routes via commands

**Design:**
- `ytcli watch` runs foreground loop:
  1. Connect IPC; `observe_property` / poll: prefer mpv JSON IPC `["observe_property", 1, "eof-reached"]` or wait for events. Practical approach for MVP:
     - Enable mpv event emission: after connect, request `{"command":["observe_property",1,"idle-active"]}` and/or read event lines.
     - Simpler MVP loop (acceptable): every 500ms `get_property idle-active` or `eof-reached`; when true and was playing, call advance logic **in-process** (same functions as `cmd_next` auto mode) then continue; if NoNext → perform stop cleanup and exit watch.
  2. If socket dead → clear watch_pid in state, exit 0.

- `ensure_running(state)`:
  - If `watch_pid` alive (kill -0 / `/proc`) → Ok
  - Else spawn: `Command::new(current_exe()).arg("watch")` with stdin/stdout/stderr null, detach; save pid

- `kill_watch(state)`: kill pid if set; clear watch_pid

Auto-advance must use **auto mode**: on NoNext → stop limpio (not error). Factor `advance_after_eof(state) -> Result<()>` used by watch.

- [ ] **Step 1: Unit test** `advance_after_eof` with fake state (last track → clears; mid → increments) without mpv
- [ ] **Step 2: Implement watch + ensure + kill**
- [ ] **Step 3: Wire ensure into play/next/prev loads; kill into stop**
- [ ] **Step 4: `cargo test` PASS**
- [ ] **Step 5: Manual note** if mpv available

**Caveat:** polling `eof-reached`/`idle-active` can false-trigger on startup idle — only arm after first successful loadfile / when `now_playing` is Some and not paused-at-start. Track `had_playback` flag in watch loop: set true when `time-pos` > 0 once; then idle-active true ⇒ eof.

---

### Task 7: README + smoke checklist

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document** add, queue/list, next, prev, prev -f, clear, status format, watch (internal), auto-advance
- [ ] **Step 2: `cargo test` + `cargo clippy --all-targets -- -D warnings`**
- [ ] **Step 3: Manual checklist** (needs mpv):

```bash
cargo run -- search "lofi" -n 5
cargo run -- play 1
cargo run -- add 2
cargo run -- add 3
cargo run -- queue
cargo run -- status
cargo run -- next
cargo run -- prev
cargo run -- prev -f
cargo run -- clear
cargo run -- stop
```

---

## Self-review (plan vs spec)

| Spec item | Task |
|-----------|------|
| play replace queue | 2, 5 |
| add enqueue | 2, 5 |
| next / prev / prev -f | 2, 5 |
| clear keep playing | 2, 5 |
| status [i/n] + time | 3, 5 |
| watch auto-next / stop at end | 6 |
| stop kills watch + clears queue | 5, 6 |
| README | 7 |

No TBD left; watch uses polling MVP with `had_playback` guard to avoid idle false positive.
