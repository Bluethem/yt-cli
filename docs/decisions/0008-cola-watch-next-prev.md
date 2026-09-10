# 0008 — Cola en state + watch + next/prev

**Fecha:** 2026-09-10  
**Estado:** Aceptada  
**Supersede:** parcialmente [0004](0004-sin-cola-mvp.md) (el MVP sin cola queda histórico)  
**Spec:** [`../superpowers/specs/2026-09-10-ytcli-queue-design.md`](../superpowers/specs/2026-09-10-ytcli-queue-design.md)

## Contexto

El MVP reproduce una sola pista (`loadfile replace`). Se necesita cola, progreso temporal, `next`/`prev`, y auto-avance al terminar.

## Decisión

1. **`play`** reemplaza la cola y reproduce; **`add`** encola sin saltar.  
2. Cola en **`state.json`** (`queue`, `current_index`); URLs con yt-dlp **al vuelo**.  
3. **`watch`** (proceso ligero) escucha `end-file` y avanza; al final de cola → stop.  
4. **`prev`**: si tiempo > 3s → seek 0; **`prev -f`** fuerza anterior.  
5. **`status`/`now`**: `[i/n]` + tiempo/duración.  
6. **`clear`**: deja solo la actual; no detiene reproducción.

Descartado para esta feature: playlist nativa de mpv como fuente de verdad; auto-next solo al invocar `status`.

## Consecuencias

- Nuevo módulo/comando `watch` y campos de state.  
- Dependencia de eventos IPC de mpv para auto-avance.  
- Base lista para una futura sesión CLI interactiva sin cambiar el modelo de cola.
