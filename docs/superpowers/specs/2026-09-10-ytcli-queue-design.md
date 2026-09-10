# ytcli — Design Spec: Cola, tiempo, next/prev

**Fecha:** 2026-09-10  
**Estado:** Aprobado  
**Extiende:** [`2026-09-10-ytcli-design.md`](./2026-09-10-ytcli-design.md)  
**ADR:** [`docs/decisions/0008-cola-watch-next-prev.md`](../../decisions/0008-cola-watch-next-prev.md)

## Resumen

Añadir **cola de reproducción**, **posición temporal** en `status`/`now`, y navegación **`next` / `prev` / `prev -f`**, manteniendo la arquitectura orquestadora (yt-dlp + mpv IPC).

La cola es **propiedad de la app** (en `state.json`), no la playlist nativa de mpv. Las URLs de audio se resuelven **al vuelo** en cada salto. Un proceso ligero **`watch`** observa `end-file` y avanza automáticamente hasta el final de la cola.

## Metas

- Encolar pistas y navegarlas con `next` / `prev`
- Auto-avance al terminar una pista; **stop limpio** al acabar la cola (sin wrap, sin loop)
- Mostrar progreso: `▶ [i/n] Title — Artist  m:ss / m:ss`
- `clear` vacía pendientes sin detener la canción actual

## No-metas (esta feature)

- Sesión interactiva tipo Claude Code / Cursor CLI (fase posterior; puede reutilizar el mismo state)
- TUI / ratatui
- Shuffle, repeat/loop, save/load de playlists con nombre
- Resolver todas las URLs por adelantado en la playlist de mpv

## Decisiones de producto (cerradas)

| Decisión | Elección |
|----------|----------|
| Cómo construir cola | `play` **reemplaza** cola y reproduce; `add` encola sin saltar |
| Fin de pista | Auto-`next`; al final de cola → stop (sin repetir) |
| `prev` default | Si `time-pos > 3s` → seek 0; si no → pista anterior |
| Forzar anterior | Solo `prev -f` / `--force` |
| `status` / `now` | `[i/n]` + tiempo actual / duración |
| `clear` | Quita pendientes; cola = solo la actual; **no** para reproducción |
| Arquitectura | Cola en state + URL al vuelo + watcher IPC |

## Arquitectura

```
ytcli
  ├─ queue   (Vec<Track> + current_index)
  ├─ youtube (yt-dlp search / resolve URL)
  ├─ player  (mpv: loadfile, seek, time-pos, duration, end-file)
  └─ watch   (hijo detach: end-file → mismo avance que next)
```

- Un solo mpv; un solo watch por usuario.
- `watch_pid` en state; si muere mpv, watch sale y limpia su pid.
- Si watch no arranca: play sigue; warning en stderr; sin auto-next hasta reintento.

## Comandos

| Comando | Comportamiento |
|---------|----------------|
| `play <target>` | `queue = [track]`, `current_index = 0`, reproduce, ensure watch |
| `add <target>` | Append a `queue`; no cambia index ni lo que suena |
| `queue` / `list` | Lista cola; marca `>` la actual |
| `next` | Si hay siguiente → index+1, URL fresca, `loadfile`; si no → error (manual) o stop (auto fin de cola) |
| `prev` | Si tiempo > 3s → `seek 0`; else index-1 + load (error si no hay anterior) |
| `prev -f` | Siempre index-1 + load (error si no hay anterior) |
| `status` / `now` | `▶ [2/5] Title — Artist  1:23 / 3:45` |
| `clear` | `queue = [queue[current]]`, index = 0; mpv sigue |
| `stop` | Para mpv, vacía cola, mata watch, limpia playback state |

**Target** (`play` / `add`): igual que MVP — entero ≥ 1 → `last_search[n-1]`; si no → query (primer resultado para play; para add igual primer resultado de esa query, o índice del search).

Índices de UI **1-based**; `current_index` interno **0-based**.

### Matiz `next` manual vs auto

- **`next` manual** sin siguiente: error amigable; no wrap; no apagar mpv si aún hay audio (caso raro si index ya es último y sigue sonando).
- **Auto (watch) al `end-file`** sin siguiente: stop limpio (quit mpv / clear playback + cola vacía o solo cleanup coherente con stop).

## Estado en disco

Ampliar `~/.cache/ytcli/state.json`:

```json
{
  "mpv_pid": 12345,
  "ipc_socket": "/home/user/.cache/ytcli/mpv.sock",
  "watch_pid": 12346,
  "current_index": 1,
  "queue": [
    { "id": "...", "title": "...", "uploader": "...", "webpage_url": "...", "duration_secs": 180 }
  ],
  "now_playing": { "...": "debe coincidir con queue[current_index] cuando hay playback" },
  "last_search": []
}
```

Invariante: con reproducción activa, `now_playing ≈ queue[current_index]` (o se deriva solo de la cola y se depreca duplicar; implementación puede unificar leyendo de `queue[current_index]`).

## Flujos

### `play`
1. Resolver track (search o índice).
2. `queue = [track]`, `current_index = 0`.
3. Resolve URL → ensure mpv → `loadfile replace`.
4. Guardar state; ensure watch.

### `add`
1. Resolver track.
2. `queue.push(track)`; save.
3. No IPC de carga.

### Auto-next (`watch`)
1. Evento `end-file` (razón normal/eof).
2. Si `current_index + 1 < queue.len()` → mismo path que `next`.
3. Si no → stop limpio.

### `prev` / `prev -f`
1. Leer `time-pos` por IPC.
2. Sin `-f` y tiempo > 3 → `seek 0`.
3. Else / con `-f` → decrementar index si ≥ 1, else error; URL + loadfile.

### `clear`
1. Si hay current: `queue = [queue[i]]`, `current_index = 0`.
2. Si no hay current: `queue = []`.
3. No hablar con mpv salvo actualizar state.

### `status` / `now`
1. require socket vivo (fallback socket path como hoy).
2. `time-pos`, `duration` vía IPC.
3. Imprimir `[current_index+1/queue.len()]` + título + tiempos formateados (incl. horas si ≥ 3600s).

## Errores

| Caso | Comportamiento |
|------|----------------|
| `next` sin siguiente (manual) | Error claro; no wrap |
| `prev`/`prev -f` sin anterior | Error; al inicio de cola |
| `add`/`play` sin resultados | Como MVP |
| Watch no arranca | Warning; play OK |
| IPC sin time-pos | Mostrar `?:??` o omitir tiempos; no crashear |
| State JSON corrupto | Seguir degradando a default + warning (comportamiento actual post-MVP harden) |

## Testing

| Capa | Qué |
|------|-----|
| Queue logic | next/prev/clear/índices; regla 3s y `-f` (unit, sin mpv) |
| Player | get time/duration helpers; seek payload |
| Watch | handler de avance con evento inyectado |
| Manual | play → add×2 → status → next/prev/`prev -f` → clear → dejar terminar última |

Sin CI contra YouTube real; sin test E2E mpv obligatorio en CI.

## Compatibilidad con MVP

- `play` deja de ser “solo replace de una pista en el vacío”: ahora **reemplaza la cola completa**.
- ADR [`0004-sin-cola-mvp.md`](../../decisions/0004-sin-cola-mvp.md) queda **superseded** por esta feature para producto; el MVP histórico permanece documentado.

## Fase posterior (fuera de este spec)

Sesión CLI tipo agente (historial de canciones por sesión, prompt persistente) reutilizando `queue` + `watch` + mismo state.
