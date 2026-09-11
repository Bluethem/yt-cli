# ytcli

CLI ligera para buscar y escuchar audio de YouTube desde la terminal, usando **yt-dlp** para búsqueda/resolución de URLs y **mpv** como reproductor en segundo plano (IPC).

## Requisitos

| Dependencia | Uso |
|-------------|-----|
| **Rust** (toolchain estable) | Compilar e instalar |
| **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** | Búsqueda y extracción de URL de audio |
| **[mpv](https://mpv.io/)** | Reproducción vía socket IPC |
| **[ffmpeg](https://ffmpeg.org/)** | Convertir a Opus 160 al descargar offline |

Comprueba que los binarios estén en el `PATH`:

```bash
which yt-dlp mpv ffmpeg
```

En Arch Linux:

```bash
sudo pacman -S yt-dlp mpv ffmpeg
```

## Instalación

Desde el directorio del proyecto:

```bash
cargo install --path .
```

O ejecutar sin instalar:

```bash
cargo run -- <comando> [args]
```

## Uso

### Buscar

```bash
ytcli search "lofi hip hop" -n 5
```

Muestra resultados numerados (1-based). Los guarda en `~/.cache/ytcli/state.json` para `play N` y `add N`.

### Reproducir

Por índice del último `search`:

```bash
ytcli play 1
```

Por consulta directa (reproduce el primer resultado):

```bash
ytcli play "lofi hip hop"
```

`play` **reemplaza** la cola completa con la pista elegida, la reproduce y arranca el auto-avance (ver más abajo).

### Cola de reproducción

| Comando | Descripción |
|---------|-------------|
| `add <target>` | Encola una pista sin interrumpir la actual. Mismo formato de `<target>` que `play` (índice del último `search` o consulta). |
| `queue` / `list` | Lista la cola; marca con `>` la pista actual. Si hay loop, muestra `🔁` en esa línea. |
| `next` | Pasa a la siguiente pista (resuelve URL al vuelo). Error si ya estás en la última. |
| `prev` | Si llevas más de 3 s en la pista → vuelve al inicio; si no → pista anterior. |
| `prev -f` / `prev --force` | Siempre salta a la pista anterior (sin reiniciar la actual). |
| `clear` | Elimina las pistas pendientes; la cola queda solo con la actual. **No** detiene la reproducción. |
| `shuffle` | Mezcla solo las pistas **pendientes** (después de la actual). La que suena no cambia. |

### Playlist de YouTube o Spotify

```bash
# YouTube: encola hasta 100 pistas (por defecto) sin interrumpir
ytcli playlist "https://www.youtube.com/playlist?list=PLxxxx"

# Spotify (playlist / álbum / track públicos): metadatos vía API → mirror YouTube
# Opción recomendada: copia `.env.example` → `.env` y rellena las claves
cp .env.example .env   # luego edita SPOTIFY_CLIENT_ID / SPOTIFY_CLIENT_SECRET

# Alternativa: exportar a mano
# export SPOTIFY_CLIENT_ID=…
# export SPOTIFY_CLIENT_SECRET=…

ytcli playlist "https://open.spotify.com/playlist/…"
ytcli playlist "https://open.spotify.com/album/…" --play
ytcli playlist "spotify:track:…" -n 1

# Reemplaza la cola y reproduce desde el primero
ytcli playlist "https://www.youtube.com/playlist?list=PLxxxx" --play

# Limitar cantidad
ytcli playlist "https://www.youtube.com/playlist?list=PLxxxx" -n 30
```

Spotify **no se stream‑ea**: se usa la API (Client Credentials) para título/artista y se busca el audio en YouTube (`ytsearch1`). Crea una app en el [Spotify Developer Dashboard](https://developer.spotify.com/dashboard). ytcli carga automáticamente un archivo **`.env`** en el directorio desde el que lo ejecutas (el `.env` no se sube a git; usa `.env.example` como plantilla).

### Playlists locales

Guarda pistas y playlists de YouTube en JSON bajo `~/.local/share/ytcli/playlists/` (metadata). La reproducción usa **archivo Opus en cache** si existe, si no **stream**. En `queue`/`show`: `[↓]` local (verde), `[~]` stream. Sin nombre explícito, el destino por defecto es **`liked`**.

```bash
ytcli save
ytcli save rock
ytcli create favorites
ytcli playlist "https://www.youtube.com/playlist?list=…" --save
ytcli playlist "https://…" --save rock
ytcli playlist "https://…" --save favorites --play   # guardar y reproducir
ytcli playlist "https://…" --save favorites --download --play
ytcli save --download
ytcli download              # pista actual → cache Opus 160
ytcli download favorites    # toda una playlist local
ytcli playlists
ytcli show liked          # lista pistas; [↓] local / [~] stream
ytcli open rock           # encola al final de la cola actual (sin reemplazar ni parar)
ytcli open rock --play    # reemplaza la cola y reproduce desde el primero
ytcli playlist-rm rock --yes
ytcli queue   # muestra 🔁 si hay loop
```

### Loop de la pista actual

```bash
ytcli loop      # al terminar, vuelve a empezar la misma
ytcli unloop    # quita el loop
```

El loop se **desactiva** automáticamente con `next`, `prev -f`, `prev` (si salta a otra pista), `play`, `playlist --play` y `stop`. Un `prev` que solo reinicia la pista (regla de 3 s) **mantiene** el loop.

Ejemplo:

```bash
ytcli search "lofi" -n 5
ytcli play 1
ytcli add 2
ytcli add 3
ytcli queue
ytcli shuffle
ytcli next
ytcli prev
ytcli prev -f
ytcli clear
```

### Auto-avance

Al reproducir (`play`, `next`, `playlist --play`, etc.), ytcli lanza en segundo plano un proceso interno **`watch`** que observa mpv vía IPC. Cuando termina la pista actual, avanza automáticamente a la siguiente (o la reinicia si hay `loop`). Al llegar al final de la cola sin loop, hace un **stop limpio** (sin repetir ni dar la vuelta).

`ytcli watch` existe como comando interno/avanzado; no hace falta invocarlo a mano en el flujo normal.

### Control de reproducción

```bash
ytcli pause
ytcli resume
ytcli volume 50    # 0–100
ytcli stop         # para mpv, vacía la cola y detiene watch
```

### Estado

```bash
ytcli now
ytcli status
```

Ambos muestran la pista actual con posición en cola y progreso temporal:

```text
▶ [2/5] Título — Artista  1:23 / 3:45
🔁 [2/5] Título — Artista  1:23 / 3:45   # con loop activo
```

Formato: `[i/n]` (índice actual / total en cola), luego título, artista y `posición / duración` (`m:ss`, o `h:mm:ss` si la pista dura ≥ 1 h). Si no hay reproducción activa, indican que no hay nada sonando.

## Ejemplo de flujo

```bash
ytcli search "jazz piano" -n 5
ytcli play 2
ytcli add 3
ytcli add 4
ytcli queue
ytcli status
ytcli next
ytcli pause
ytcli resume
ytcli volume 40
ytcli stop
```

## Datos locales

- Estado: `~/.cache/ytcli/state.json` (última búsqueda, cola, índice actual, pista en reproducción, PID de mpv y de watch)
- Playlists: `~/.local/share/ytcli/playlists/<nombre>.json` (metadata persistente)
- Audio offline: `~/.cache/ytcli/audio/<id>.opus` + `index.json` (compartido entre playlists)
- Socket IPC: `~/.cache/ytcli/mpv.sock`

## Desarrollo

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Los tests unitarios no requieren `mpv` ni red; usan fixtures y mocks donde aplica.

### Checklist manual (smoke, requiere mpv + yt-dlp)

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

## Licencia

MIT
