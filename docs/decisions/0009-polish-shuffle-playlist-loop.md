# 0009 — Pulido: shuffle, playlist YouTube, loop

**Fecha:** 2026-09-10  
**Estado:** Aceptada  
**Spec:** [`../superpowers/specs/2026-09-10-ytcli-polish-design.md`](../superpowers/specs/2026-09-10-ytcli-polish-design.md)

## Contexto

Antes de TUI, faltan: mezclar cola pendiente, importar playlists YT, y loop de la pista actual. Spotify es deseable pero los bots de Discord solo usan Spotify como metadatos y reproducen vía mirror (YouTube).

## Decisión

1. **`shuffle`**: solo pendientes; la actual no se mueve.  
2. **`playlist <url>`**: YouTube vía yt-dlp flat; encola; **`--play`** reemplaza y reproduce; **`-n`** default 100.  
3. **`loop` / `unloop`**: flag `loop_current`; watch reinicia; se rompe con next / prev-f / prev a otra pista / play / playlist --play / stop.  
4. Spotify **aplazado** (mismo patrón mirror que Discord/LavaSrc).  
5. TUI **después** de este pulido.

## Consecuencias

- Cambios acotados en `youtube`, `queue`, `watch`, `commands`, `state`.  
- Base lista para TUI sin bloquear por playlists YT / shuffle / loop.
