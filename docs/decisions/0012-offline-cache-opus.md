# 0012 — Cache offline Opus + resolve stream/file

**Fecha:** 2026-09-11  
**Estado:** Aceptada  
**Spec:** [`../superpowers/specs/2026-09-11-ytcli-offline-cache-design.md`](../superpowers/specs/2026-09-11-ytcli-offline-cache-design.md)

## Contexto

Se quiere escuchar offline con buen ratio calidad/tamaño, sin duplicar archivos por playlist, conviviendo con el stream actual.

## Decisión

1. **yt-dlp `bestaudio` → ffmpeg Opus 160 VBR** en `~/.cache/ytcli/audio/<id>.opus` + `index.json`.  
2. **Resolve:** preferir archivo válido; si no → stream.  
3. **`download`** + flags `--download` en `save` / `playlist`.  
4. **UI:** `[↓]`+verde / `[~]`; `NO_COLOR` / `--no-color`.  
5. Playlists JSON sin paths; cache es la fuente de verdad por id.

## Consecuencias

- Nueva dependencia runtime: **ffmpeg**.  
- Una canción en N playlists = un `.opus`.  
- Watch/play/next usan el mismo resolve.
