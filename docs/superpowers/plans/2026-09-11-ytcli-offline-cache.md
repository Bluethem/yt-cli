# ytcli Offline Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prefer local Opus cache when playing; download via yt-dlp bestaudio → ffmpeg Opus 160; shared index by YouTube id; queue markers `[↓]`/`[~]` with color.

**Architecture:** `cache` module owns `~/.cache/ytcli/audio/` + `index.json`. `download` runs yt-dlp+ffmpeg. `resolve_playback` returns Path or Url. Commands/watch use resolve. Display helpers for prefixes/colors.

**Tech Stack:** Rust existing + yt-dlp + ffmpeg + mpv. No new audio crates.

## Global Constraints

- Opus 160 VBR default; yt-dlp `-f bestaudio` then ffmpeg
- Prefer file if exists+valid else stream
- `download` + `--download` on save/playlist
- Cache: `cache_dir()/ytcli/audio/<id>.opus` + `index.json`
- Playlists JSON unchanged (no local_path)
- UI: `[↓]`+green local, `[~]` stream; `NO_COLOR` / `--no-color`
- Messages in Spanish
- Unit tests without network for cache/resolve/display

## File map

| File | Role |
|------|------|
| `src/cache.rs` | index CRUD, paths, has/get/put |
| `src/download.rs` | yt-dlp + ffmpeg pipeline |
| `src/display.rs` | markers, ANSI, no_color |
| `src/playback.rs` | `PlaybackSource` + resolve |
| `src/cli.rs` | Download, --download, --no-color on Cli |
| `src/commands.rs` / `watch.rs` | wire resolve + download |
| `src/lib.rs` | mods |
| `README.md` + ADR 0012 | docs |

---

### Task 1: cache module + tests

**Files:** Create `src/cache.rs`; modify `lib.rs`

- [ ] TDD: put/get/has; atomic index; missing file → not has
- [ ] `AudioCache::at(root)` / `system()` under `AppState::cache_dir()?.join("audio")`
- [ ] Commit

### Task 2: display + playback resolve

**Files:** `src/display.rs`, `src/playback.rs`

- [ ] `format_track_line(..., is_local)` with `[↓]`/`[~]` and green if color
- [ ] `PlaybackSource::{Local(PathBuf), Stream(String)}` + `resolve(track, cache)`
- [ ] Tests NO_COLOR / prefer local
- [ ] Commit

### Task 3: download pipeline

**Files:** `src/download.rs`, `error.rs` if needed

- [ ] `download_track(track, cache)`: skip if has; yt-dlp -f bestaudio -o temp; ffmpeg -b:a 160k; put index; cleanup
- [ ] `ensure_ffmpeg`
- [ ] Tests: skip-if-cached only (no real network in CI)
- [ ] Commit

### Task 4: CLI + commands + watch

**Files:** `cli.rs`, `commands.rs`, `watch.rs`, `main` if global color

- [ ] `Download { name: Option<String> }`, playlist/save `--download`, Cli `--no-color`
- [ ] Wire download; save/playlist flags; all loadfile via resolve
- [ ] Update queue/show formatting
- [ ] Commit

### Task 5: docs + verify

- [ ] README deps ffmpeg; ADR 0012; roadmap note
- [ ] `cargo test` + clippy
- [ ] Commit
