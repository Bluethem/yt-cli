# 0011 — Playlists locales en JSON (stream only)

**Fecha:** 2026-09-11  
**Estado:** Aceptada  
**Spec:** [`../superpowers/specs/2026-09-11-ytcli-local-playlists-design.md`](../superpowers/specs/2026-09-11-ytcli-local-playlists-design.md)

## Contexto

Tras cola, pulido y Spotify mirror, se quiere persistir pistas y playlists de YouTube en disco sin descarga offline ni Premium. La cola de sesión (`state.json`) no debe mezclarse con una librería reutilizable.

## Decisión

1. **Un JSON por playlist** bajo `data_dir()/ytcli/playlists/<slug>.json`; metadata only (shape de `Track`).  
2. **Default `liked`** cuando no se nombra destino (`save`, `playlist --save` sin nombre).  
3. **Reproducción = stream** (yt-dlp + mpv al vuelo); sin archivos de audio en esta fase.  
4. **`queue` muestra loop** — pista actual con `>` y `🔁` si `loop_current`.  
5. Comandos: `save`, `playlists`, `open`, `playlist-rm`; flag `--save` en `playlist`.  
6. **MP3 / download** documentado como fase posterior (yt-dlp audio + re-encode).

## Consecuencias

- Cola (cache) y librería (data dir) separadas; `open` copia tracks a la cola.  
- Escritura atómica (temp + rename), dedup por `id` al append.  
- Sin SQLite ni crates de audio nuevos.  
- TUI sigue siendo fase posterior.
