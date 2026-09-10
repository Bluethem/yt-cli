# ytcli — Design Spec: Pulido pre-TUI (shuffle, playlist YT, loop)

**Fecha:** 2026-09-10  
**Estado:** Aprobado  
**Extiende:** [`2026-09-10-ytcli-queue-design.md`](./2026-09-10-ytcli-queue-design.md)  
**ADR:** [`docs/decisions/0009-polish-shuffle-playlist-loop.md`](../../decisions/0009-polish-shuffle-playlist-loop.md)

## Resumen

Antes de la TUI (estilo rmpc + cmdline), pulir la CLI con:

1. **`shuffle`** — mezclar solo lo pendiente de la cola  
2. **`playlist <url>`** — importar playlist de **YouTube** (encolar; `--play` reemplaza y reproduce)  
3. **`loop` / `unloop`** — repetir la pista actual; se rompe con `next`, `prev -f`, cambio a otra pista vía `prev`, y `play` / `playlist --play`

Spotify queda **fuera** de este corte (futuro: metadatos API → mirror `ytsearch`, como bots de Discord).

## Metas

- Mezclar pendientes sin alterar la canción en curso  
- Encolar playlists de YouTube con un comando  
- Loop de una sola pista con escape claro vía navegación  

## No-metas

- TUI / ratatui  
- Spotify / Apple Music  
- Loop de cola completa / repeat-all  
- Deduplicación de tracks al importar  

## Decisiones cerradas

| Tema | Elección |
|------|----------|
| Playlists | Solo YouTube ahora (`yt-dlp --flat-playlist`) |
| `playlist` | Encola; `playlist --play` reemplaza + reproduce |
| `shuffle` | Solo pendientes (`queue[current+1..]`) |
| Loop | Solo pista actual; flag `loop_current` |
| Romper loop | `next`, `prev -f`, `prev` si cambia de pista, `play`, `playlist --play` |
| `prev` solo seek 0 (regla 3s) | **Mantiene** loop |
| Spotify | Fase posterior (mirror YouTube) |
| TUI | Tras este pulido; híbrido paneles + cmdline |

## Arquitectura

```
ytcli
  ├─ youtube  → fetch_playlist(url, limit)
  ├─ queue    → shuffle_pending(); loop flag helpers
  ├─ watch    → si loop_current: reiniciar pista; else auto-next
  └─ commands → shuffle, playlist, loop, unloop
```

Sin deps nuevas de audio. Misma orquestación yt-dlp + mpv.

## Comandos

### `shuffle`
- Requiere cola con al menos un pendiente tras `current_index`.
- Reordena aleatoriamente `queue[i+1..]`; `current_index` y pista actual intactos.
- Sin pendientes: mensaje amigable, exit 0.

### `playlist <url> [--play] [-n LIMIT]`
- Extrae entradas con yt-dlp flat JSON → `Vec<Track>`.
- Default **sin** `--play`: append a `queue` (como `add` masivo).
- Con `--play`: `queue = tracks`, `current_index = 0`, loadfile primero, ensure watch, `loop_current = false`.
- `--limit` / `-n`: tope de pistas (default **100**).
- URL: playlist YT (`list=`) o URL de watch con `list=`.
- Items inválidos: se omiten; warning con conteo ok/fail.
- Duplicados permitidos.

### `loop` / `unloop`
- `loop`: `loop_current = true` si hay pista actual; status muestra `🔁` / `[loop]`.
- `unloop`: `loop_current = false`.
- Con loop ON, `watch` ante fin de pista: seek 0 o reload misma URL (misma `webpage_url`), **no** avanza cola.
- Clear `loop_current` en: `next`, `prev -f`, `prev` cuando `GoTo` otra pista, `play`, `playlist --play`, `stop`.

## Estado

Ampliar `state.json`:

```json
{
  "loop_current": false
}
```

`#[serde(default)]` → `false` en estados legacy.

## Errores

| Caso | Comportamiento |
|------|----------------|
| shuffle sin pendientes | Mensaje, Ok |
| playlist URL inválida / 0 tracks | Err |
| playlist parcial | Encola válidos + warning |
| loop sin now_playing | Err amigable |
| unloop ya off | Mensaje, Ok |

## Testing

- Unit `shuffle_pending`  
- Unit/watch: loop reinicia vs next  
- Unit: next/prev-f limpian loop  
- Fixture playlist JSONL  
- Manual: playlist URL → shuffle → loop → next  

## Roadmap siguiente

1. Este pulido (implementación)  
2. TUI híbrida (paneles tipo rmpc + línea de comandos)  
3. Spotify mirror (API/client credentials → ytsearch → cola)  
