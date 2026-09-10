# 0002 — Stack: Rust + yt-dlp + mpv

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Objetivo: CLI ligera. Opciones de player: rodio puro, mpv, o híbridos.

## Decisión

- **Rust** para la CLI y el estado.
- **yt-dlp** para search y `-f bestaudio -g`.
- **mpv** con `--no-video` y `--input-ipc-server` para reproducir y controlar.

Descartado en MVP: rodio/cpal como decoder principal.

## Consecuencias

- Runtime requiere `yt-dlp` y `mpv` en `PATH`.
- Menos código de codecs en el repo; mejor estabilidad con streams de YouTube.
- Control real via IPC (`pause`, `volume`, `loadfile`).
