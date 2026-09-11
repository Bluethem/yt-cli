# ytcli Local Playlists + Loop-in-Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mostrar loop en `ytcli queue` y añadir playlists locales en JSON (save / import YT / list / open / rm) según `docs/superpowers/specs/2026-09-11-ytcli-local-playlists-design.md`.

**Architecture:** Nuevo módulo `playlists` (un JSON por lista bajo XDG data); la cola en `state.json` sigue siendo sesión. Comandos CLI cablean save/open/list/rm y `playlist --save`. Sin descarga MP3 ni TUI.

**Tech Stack:** Rust existente (`clap`, `serde_json`, `dirs`, `tempfile` en tests). Sin crates de audio nuevos. Timestamps RFC3339 con helper mínimo (segundos UNIX formateados o `chrono` solo si el helper a mano no es viable — preferir **sin** dependencia nueva: campos `created_at` / `updated_at` como `String` ISO vía formateo simple UTC, ver Task 2).

## Global Constraints

- Mensajes CLI en **español**
- Default playlist name: `liked`
- `save` / `save <nombre>`: append + dedup por `Track.id`
- `playlist --save` (sin nombre): append+dedup → `liked`
- `playlist --save <nombre>`: **replace** tracks + `source = url`
- Playlists en `dirs::data_dir()/ytcli/playlists/<slug>.json` (no mezclar con cache/`state.json`)
- `playlist-rm` **requiere** `--yes`
- `open` limpia `loop_current`; `--play` reproduce índice 0
- Sin red en unit tests de playlists
- No MP3 / TUI / instalador en este plan

## File map

| File | Responsibility |
|------|----------------|
| `src/playlists.rs` | slug, store, load/save atómico, append/replace/list/delete |
| `src/error.rs` | variantes InvalidPlaylistName, PlaylistNotFound, NoCurrentTrack (o reutilizar NotPlaying) |
| `src/commands.rs` | cmd_save, cmd_playlists, cmd_open, cmd_playlist_rm; queue marker; playlist --save |
| `src/cli.rs` | Save, Playlists, Open, PlaylistRm; Playlist.save; tests parse |
| `src/lib.rs` | `pub mod playlists` |
| `docs/decisions/0011-*.md` + README | ADR |
| `README.md` | documentación de comandos |
| Spec | marcar Estado: Aprobado |

---

### Task 1: Marker de loop en `queue`

**Files:**
- Modify: `src/commands.rs` (`cmd_queue` y helper testeable)
- Test: tests en `src/commands.rs` o helper en `src/queue.rs` si preferís — **usar helper en `commands` con `#[cfg(test)]` o función `pub(crate) fn format_queue_line`**

**Interfaces:**
- Consumes: `AppState.{queue,current_index,loop_current}`, `Track`
- Produces: `format_queue_line(index_1based, track, is_current, loop_on) -> String`

- [ ] **Step 1: Write the failing test**

Añadir en `src/commands.rs` (módulo de tests existente o nuevo):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test format_queue_line_marks_loop_on_current -- --nocapture`  
Expected: FAIL (función inexistente o assert)

- [ ] **Step 3: Write minimal implementation**

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test format_queue_line_marks_loop_on_current`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/commands.rs
git commit -m "$(cat <<'EOF'
feat: show loop marker on current track in queue

EOF
)"
```

---

### Task 2: Errores + módulo `playlists` (slug, store, append, replace)

**Files:**
- Create: `src/playlists.rs`
- Modify: `src/lib.rs`, `src/error.rs`
- Test: unit tests dentro de `src/playlists.rs` con `tempfile`

**Interfaces:**
- Consumes: `Track`, `YtcliError`, `Result`
- Produces:
  - `pub const DEFAULT_NAME: &str = "liked";`
  - `pub fn validate_name(name: &str) -> Result<()>;`
  - `pub fn slugify(name: &str) -> Result<String>;` (tras validate; lowercase, espacios→`-`, solo `[a-z0-9_-]`)
  - `pub struct PlaylistStore { root: PathBuf }`
  - `impl PlaylistStore { pub fn at(root: PathBuf) -> Self; pub fn system() -> Result<Self>; ... }`
  - `pub struct LocalPlaylist { name, created_at, updated_at, source: Option<String>, tracks: Vec<Track> }`
  - `append_track(&self, name: &str, track: Track) -> Result<LocalPlaylist>`
  - `append_tracks(&self, name: &str, tracks: Vec<Track>) -> Result<LocalPlaylist>`
  - `replace_tracks(&self, name: &str, tracks: Vec<Track>, source: Option<String>) -> Result<LocalPlaylist>`
  - `load(&self, name: &str) -> Result<LocalPlaylist>`
  - `list(&self) -> Result<Vec<(String, usize)>>`  // (name, len)
  - `delete(&self, name: &str) -> Result<()>`
  - escritura atómica temp+rename como `AppState::save_to`

**Errores nuevos en `error.rs`:**

```rust
#[error("nombre de playlist inválido: {0}")]
InvalidPlaylistName(String),

#[error("no existe la playlist `{0}`; prueba `ytcli playlists`")]
PlaylistNotFound(String),
```

`save` sin pista actual → reutilizar `YtcliError::NotPlaying`.

- [ ] **Step 1: Write failing tests** (en `playlists.rs`)

```rust
#[test]
fn reject_path_traversal_names() {
    assert!(validate_name("..").is_err());
    assert!(validate_name("a/b").is_err());
    assert!(validate_name("").is_err());
    assert!(validate_name("ok-list").is_ok());
}

#[test]
fn append_dedup_by_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = PlaylistStore::at(dir.path().to_path_buf());
    let t1 = Track { id: "1".into(), title: "A".into(), uploader: "U".into(), webpage_url: "u".into(), duration_secs: None };
    let t1b = Track { id: "1".into(), title: "A2".into(), uploader: "U".into(), webpage_url: "u".into(), duration_secs: None };
    let t2 = Track { id: "2".into(), title: "B".into(), uploader: "U".into(), webpage_url: "u".into(), duration_secs: None };
    store.append_track("liked", t1).unwrap();
    store.append_track("liked", t1b).unwrap();
    let pl = store.append_track("liked", t2).unwrap();
    assert_eq!(pl.tracks.len(), 2);
    assert_eq!(pl.tracks[0].title, "A"); // no reemplaza título en dedup
}

#[test]
fn replace_sets_source_and_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let store = PlaylistStore::at(dir.path().to_path_buf());
    let t1 = Track { id: "1".into(), title: "A".into(), uploader: "U".into(), webpage_url: "u".into(), duration_secs: None };
    store.append_track("rock", t1.clone()).unwrap();
    let t2 = Track { id: "9".into(), title: "Z".into(), uploader: "U".into(), webpage_url: "u".into(), duration_secs: None };
    let pl = store.replace_tracks("rock", vec![t2], Some("https://youtube.com/playlist?list=X".into())).unwrap();
    assert_eq!(pl.tracks.len(), 1);
    assert_eq!(pl.tracks[0].id, "9");
    assert_eq!(pl.source.as_deref(), Some("https://youtube.com/playlist?list=X"));
}

#[test]
fn delete_missing_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let store = PlaylistStore::at(dir.path().to_path_buf());
    let err = store.delete("nope").unwrap_err();
    assert!(matches!(err, YtcliError::PlaylistNotFound(_)));
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test --lib playlists::`  
Expected: compile fail / FAIL

- [ ] **Step 3: Implement `playlists.rs` + wire `lib.rs` + errors**

`PlaylistStore::system()`:

```rust
pub fn system() -> Result<Self> {
    let base = dirs::data_dir().ok_or_else(|| YtcliError::StateIo {
        path: PathBuf::from("(data_dir)"),
        detail: "dirs::data_dir() devolvió None".into(),
    })?;
    Ok(Self::at(base.join("ytcli").join("playlists")))
}
```

Timestamps: función `fn now_rfc3339() -> String` usando `std::time::SystemTime` y formateo manual UTC **o** guardar `humantime`-like simple:

```rust
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // suficiente para v1 si no queréis chrono: epoch como string decimal
    // PREFERIDO para cumplir spec ISO: añadir chrono = { version = "0.4", default-features = false, features = ["clock"] }
    // y: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    secs.to_string()
}
```

**Decisión del implementador:** si el spec exige ISO literal, añadir `chrono` mínimo; si no, documentar epoch en ADR. **Preferir chrono clock-only** para cumplir el JSON del spec.

`slugify`: tras `validate_name`, mapear a filename `"{slug}.json"`. Display `name` en JSON = nombre pedido por el usuario (trim), no solo slug.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test --lib playlists::`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/playlists.rs src/lib.rs src/error.rs Cargo.toml Cargo.lock
git commit -m "$(cat <<'EOF'
feat: add local playlist JSON store with dedup

EOF
)"
```

---

### Task 3: CLI — parse Save / Playlists / Open / PlaylistRm / playlist --save

**Files:**
- Modify: `src/cli.rs`

**Interfaces:**
- Produces variantes:

```rust
Save {
    /// Destino (default liked)
    name: Option<String>,
},
Playlists,
Open {
    name: String,
    #[arg(long)]
    play: bool,
},
PlaylistRm {
    name: String,
    #[arg(long)]
    yes: bool,
},
// en Playlist:
/// Guardar local: `--save` → liked (append); `--save NOMBRE` → replace
#[arg(long, num_args = 0..=1, default_missing_value = "__LIKED_APPEND__")]
save: Option<String>,
```

Mejor patrón clap (distinguir bare `--save` vs `--save name`):

```rust
#[arg(long, num_args = 0..=1, value_name = "NOMBRE")]
save: Option<Option<String>>,
```

- Ausente → `None`
- `--save` → `Some(None)` → append liked
- `--save rock` → `Some(Some("rock"))` → replace

- [ ] **Step 1: Write failing parse tests**

```rust
#[test]
fn parse_save_default_and_named() {
    let cli = Cli::try_parse_from(["ytcli", "save"]).unwrap();
    match cli.command {
        Commands::Save { name: None } => {}
        _ => panic!(),
    }
    let cli = Cli::try_parse_from(["ytcli", "save", "rock"]).unwrap();
    match cli.command {
        Commands::Save { name: Some(n) } => assert_eq!(n, "rock"),
        _ => panic!(),
    }
}

#[test]
fn parse_playlist_save_optional_name() {
    let cli = Cli::try_parse_from(["ytcli", "playlist", "https://x", "--save"]).unwrap();
    match cli.command {
        Commands::Playlist { save: Some(None), .. } => {}
        other => panic!("{other:?}"),
    }
    let cli = Cli::try_parse_from(["ytcli", "playlist", "https://x", "--save", "rock"]).unwrap();
    match cli.command {
        Commands::Playlist { save: Some(Some(n)), .. } => assert_eq!(n, "rock"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn parse_playlist_rm_requires_yes_flag_present_in_argv() {
    let cli = Cli::try_parse_from(["ytcli", "playlist-rm", "rock", "--yes"]).unwrap();
    match cli.command {
        Commands::PlaylistRm { name, yes: true } => assert_eq!(name, "rock"),
        _ => panic!(),
    }
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --lib cli::tests::parse_save`  
Expected: FAIL

- [ ] **Step 3: Implement enum + args**

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "$(cat <<'EOF'
feat: CLI for local playlists and playlist --save

EOF
)"
```

---

### Task 4: Wire commands (`save`, `playlists`, `open`, `playlist-rm`, `playlist --save`)

**Files:**
- Modify: `src/commands.rs` (`run` match + handlers)
- Modify: `src/main.rs` solo si el match no está centralizado en `commands::run`

**Interfaces:**
- Consumes: `PlaylistStore::system()`, helpers Task 2, `queue::*`, `load_queue_index` existente

- [ ] **Step 1: Implement handlers** (tests de integración opcionales; lógica de save testeada vía store)

```rust
fn current_track(state: &AppState) -> Result<Track> {
    if let Some(idx) = state.current_index {
        if let Some(t) = state.queue.get(idx) {
            return Ok(t.clone());
        }
    }
    state.now_playing.clone().ok_or(YtcliError::NotPlaying)
}

fn cmd_save(name: Option<String>) -> Result<()> {
    let state = AppState::load()?;
    let track = current_track(&state)?;
    let dest = name.unwrap_or_else(|| playlists::DEFAULT_NAME.to_string());
    let store = PlaylistStore::system()?;
    let pl = store.append_track(&dest, track)?;
    println!(
        "Guardado en «{}» ({} pistas).",
        pl.name,
        pl.tracks.len()
    );
    Ok(())
}

fn cmd_playlists() -> Result<()> {
    let store = PlaylistStore::system()?;
    let list = store.list()?;
    if list.is_empty() {
        println!("No hay playlists locales. Prueba `ytcli save` o `playlist --save`.");
        return Ok(());
    }
    for (name, n) in list {
        println!("{name}  ({n})");
    }
    Ok(())
}

fn cmd_open(name: &str, play: bool) -> Result<()> {
    let store = PlaylistStore::system()?;
    let pl = store.load(name)?;
    if pl.tracks.is_empty() {
        println!("La playlist «{}» está vacía.", pl.name);
        return Ok(());
    }
    let mut state = AppState::load()?;
    queue::clear_loop(&mut state);
    if play {
        // mismo patrón que cmd_playlist --play (mpv + primer track)
        // ...
    } else {
        state.queue = pl.tracks;
        state.current_index = None;
        state.now_playing = None;
        state.save()?;
        println!("Cargadas {} pistas desde «{}». Usa `ytcli play 1` o `open --play`.", state.queue.len(), pl.name);
    }
    Ok(())
}

fn cmd_playlist_rm(name: &str, yes: bool) -> Result<()> {
    if !yes {
        return Err(YtcliError::InvalidPlaylistName(
            // mejor variante dedicada o mensaje:
            // usar error genérico — añadir:
            // #[error("confirma el borrado con `ytcli playlist-rm {0} --yes`")]
            // PlaylistRmNeedsYes(String),
            name.to_string(),
        ));
    }
    // Preferir error dedicado PlaylistRmNeedsYes
    PlaylistStore::system()?.delete(name)?;
    println!("Playlist «{name}» eliminada.");
    Ok(())
}
```

Añadir `PlaylistRmNeedsYes` en error.rs si no quedó en Task 2.

En `cmd_playlist`, tras obtener `tracks`:

```rust
if let Some(save_opt) = save {
    let store = PlaylistStore::system()?;
    match save_opt {
        None => {
            let pl = store.append_tracks(playlists::DEFAULT_NAME, tracks.clone())?;
            println!("Guardadas en «{}» ({} pistas).", pl.name, pl.tracks.len());
        }
        Some(name) => {
            let pl = store.replace_tracks(&name, tracks.clone(), Some(url.to_string()))?;
            println!("Playlist local «{}» actualizada ({} pistas).", pl.name, pl.tracks.len());
        }
    }
}
// luego encolar / play como hoy
```

Firma: `fn cmd_playlist(url: &str, play: bool, limit: usize, save: Option<Option<String>>) -> Result<()>`

- [ ] **Step 2: `cargo test` + `cargo build`**

Expected: PASS / OK

- [ ] **Step 3: Commit**

```bash
git add src/commands.rs src/error.rs
git commit -m "$(cat <<'EOF'
feat: wire save, open, playlists, and playlist --save

EOF
)"
```

---

### Task 5: Docs — README, ADR 0011, spec aprobado

**Files:**
- Modify: `README.md` (sección playlists locales + queue loop)
- Create: `docs/decisions/0011-local-playlists-json.md`
- Modify: `docs/decisions/README.md` (fila 0011)
- Modify: `docs/superpowers/specs/2026-09-11-ytcli-local-playlists-design.md` → `Estado: Aprobado`
- Modify: `docs/decisions/0007-roadmap-cli-luego-tui.md` — añadir fase 1.8 playlists locales si encaja en una línea

- [ ] **Step 1: Escribir ADR** (decisión: JSON por archivo, liked default, stream only, MP3 futuro)

- [ ] **Step 2: Actualizar README** con ejemplos:

```bash
ytcli save
ytcli save rock
ytcli playlist "https://www.youtube.com/playlist?list=…" --save
ytcli playlist "https://…" --save rock
ytcli playlists
ytcli open rock
ytcli open rock --play
ytcli playlist-rm rock --yes
ytcli queue   # muestra 🔁 si hay loop
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/
git commit -m "$(cat <<'EOF'
docs: local playlists ADR and README

EOF
)"
```

---

### Task 6: Verificación final

- [ ] **Step 1:** `cargo test`  
  Expected: all PASS

- [ ] **Step 2:** `cargo clippy --all-targets -- -D warnings`  
  Expected: clean

- [ ] **Step 3:** Smoke manual (si hay red): `save` / `playlists` / `open` / `queue` con loop

---

## Spec coverage check

| Spec item | Task |
|-----------|------|
| Loop marker en queue | 1 |
| JSON bajo data_dir | 2 |
| liked default, save / save name | 3–4 |
| playlist --save / --save name | 3–4 |
| playlists / open / playlist-rm --yes | 3–4 |
| dedup / replace+source | 2 |
| open limpia loop | 4 |
| atomic write | 2 |
| tests sin red | 2 |
| no MP3 | — out of scope |
| README / ADR | 5 |

## Placeholder / consistency self-review

- Nombres: `PlaylistStore`, `DEFAULT_NAME = "liked"`, `save: Option<Option<String>>` consistentes en Tasks 2–4.
- `PlaylistRmNeedsYes` debe añadirse en Task 2 o 4 (no dejar InvalidPlaylistName abusado).
- Timestamps: preferir `chrono` clock-only para RFC3339 del spec.
