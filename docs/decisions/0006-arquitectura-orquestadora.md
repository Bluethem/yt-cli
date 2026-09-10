# 0006 — Arquitectura: CLI orquestadora

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Tres enfoques comparados en brainstorming:

1. CLI orquestadora + mpv IPC  
2. Daemon Rust + rodio  
3. Wrappers shell sin IPC estructurado  

## Decisión

**Enfoque 1:** Rust orquesta yt-dlp y mpv; módulos `cli`, `youtube`, `player`, `state`, `error`.

## Consecuencias

- Encaja con bajo consumo y con daemon + control.
- TUI futura reutiliza los mismos módulos.
- El proyecto no “posee” el pipeline de audio; depende de herramientas del sistema.
