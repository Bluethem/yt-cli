# ytcli — Design Spec: Spotify playlist/album/track → YouTube mirror

**Fecha:** 2026-09-10  
**Estado:** Aprobado  
**Extiende:** [`2026-09-10-ytcli-polish-design.md`](./2026-09-10-ytcli-polish-design.md)  
**ADR:** [`docs/decisions/0010-spotify-mirror-youtube.md`](../../decisions/0010-spotify-mirror-youtube.md)

## Resumen

Ampliar `playlist <url>` para aceptar enlaces de **Spotify** (playlist, álbum, track) usando **Client Credentials**. Spotify solo aporta metadatos; el audio se resuelve con `yt-dlp` vía `ytsearch1:"artista título"` (mismo patrón que bots de Discord / LavaSrc).

## Metas

- Pegar URL de Spotify pública y encolar / `--play` como con YouTube  
- Reutilizar cola, watch, shuffle, loop  
- Credenciales solo por env vars  

## No-metas

- Stream nativo de Spotify  
- OAuth de usuario / playlists privadas / Liked Songs  
- Matching por ISRC  
- TUI  

## Decisiones cerradas

| Tema | Elección |
|------|----------|
| Auth | Client Credentials (`SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET`) |
| Tipos URL | Playlist + álbum + track (también URI `spotify:…`) |
| Mirror | `ytsearch1:"artista título"` (primer resultado) |
| UX comando | Mismo `playlist` (detectar host); no comando aparte |
| Límite `-n` | Tras expandir metadatos Spotify, tomar los primeros N |
| Fallos parciales | Omitir + warning; 0 OK → Err |

## Arquitectura

```
playlist <url>
  ├─ YouTube  → youtube::fetch_playlist
  └─ Spotify  → spotify::expand(url, limit)
                  ├─ token (client_credentials)
                  ├─ fetch tracks metadata
                  └─ for each: yt-dlp ytsearch1 → Track
                → enqueue / --play (commands existentes)
```

Módulo nuevo: `src/spotify.rs` (parse URL, token, fetch, mirror helper que llama a `YtDlp`).

Token: obtener por invocación (sin cache persistente en MVP).

## Comandos

Sin cambios de superficie salvo que `playlist` acepte Spotify:

```bash
ytcli playlist "https://open.spotify.com/playlist/…"
ytcli playlist "https://open.spotify.com/album/…" --play
ytcli playlist "https://open.spotify.com/track/…" -n 1
ytcli playlist "spotify:playlist:…"
```

Progreso opcional en stderr: `Resolviendo 12/40…`.

`--play` limpia `loop_current` como hoy.

## Errores

| Caso | Comportamiento |
|------|----------------|
| Env vars ausentes | Err + hint Dashboard / export |
| Auth/API error | Err con detalle breve |
| Recurso privado/404 | Err amigable |
| 0 mirrors | Err |
| Parcial | Encola OK + warning omitidas |

## Deps

- `reqwest` con `blocking`, `json` (TLS default)

## Testing

- Parse URL/URI → tipo + id  
- Construcción de query de búsqueda desde metadatos  
- Fixtures JSON token + playlist items  
- Sin red real en CI  

## Setup usuario

1. Crear app en [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)  
2. `export SPOTIFY_CLIENT_ID=…`  
3. `export SPOTIFY_CLIENT_SECRET=…`  
