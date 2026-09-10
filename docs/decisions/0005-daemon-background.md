# 0005 — Daemon en background + IPC

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Si `play` bloquea la terminal, no hay forma natural de `pause`/`volume` como comandos aparte.

## Decisión

**mpv en background** (daemon de facto). Cada invocación de `ytcli` que controla audio habla por el socket IPC. Estado (pid, socket, now_playing) en `~/.cache/ytcli/state.json`.

Descartado para MVP: modo solo foreground; híbrido `--detach` (se puede añadir después).

## Consecuencias

- Hay que detectar IPC muerto / pid zombie y limpiar state.
- Una sola instancia de mpv por usuario.
