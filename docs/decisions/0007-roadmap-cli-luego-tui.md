# 0007 — Roadmap: CLI primero, TUI después

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Se quiere producto usable pronto, con UI rica más adelante.

## Decisión

1. **Fase 1:** CLI básica (MVP).  
2. **Fase 1.5:** Cola + tiempo + next/prev + watch ([0008](0008-cola-watch-next-prev.md)).  
3. **Fase 2:** TUI y/o sesión CLI interactiva, sobre los mismos backends.

## Consecuencias

- No introducir ratatui en el MVP.
- APIs internas de `youtube` / `player` / `state` deben ser usables sin terminal interactiva.
