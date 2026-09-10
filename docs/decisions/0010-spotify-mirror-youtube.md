# 0010 — Spotify mirror vía YouTube (Client Credentials)

**Fecha:** 2026-09-10  
**Estado:** Aceptada  
**Spec:** [`../superpowers/specs/2026-09-10-ytcli-spotify-design.md`](../superpowers/specs/2026-09-10-ytcli-spotify-design.md)

## Contexto

Se quieren playlists de Spotify sin stream oficial. Los bots de Discord resuelven metadatos con la API y reproducen un mirror (YouTube).

## Decisión

1. **Client Credentials** con `SPOTIFY_CLIENT_ID` / `SPOTIFY_CLIENT_SECRET`.  
2. Soporte **playlist + álbum + track** (URL y URI).  
3. Mirror: **`ytsearch1:"artista título"`**.  
4. Reusar comando **`playlist`** (detección por URL), no comando aparte.  
5. `-n` limita tras expandir; fallos parciales con warning.

## Consecuencias

- Dep `reqwest` (blocking).  
- Solo contenido Spotify **público**.  
- Imports pueden ser lentos (1 búsqueda YT por tema).  
- TUI sigue siendo fase posterior.
