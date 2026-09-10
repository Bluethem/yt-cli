# 0007 — Roadmap: CLI primero, TUI después

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Se quiere producto usable pronto, con UI rica más adelante.

## Decisión

1. **Fase 1:** CLI básica (este spec).  
2. **Fase 2:** TUI (`ratatui` + `crossterm`) + cola, sobre los mismos backends.

## Consecuencias

- No introducir ratatui en el MVP.
- APIs internas de `youtube` / `player` / `state` deben ser usables sin terminal interactiva.
