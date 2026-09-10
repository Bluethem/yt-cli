# 0004 — Sin cola en el MVP

**Fecha:** 2026-09-10  
**Estado:** Superseded por [0008](0008-cola-watch-next-prev.md) (histórico del MVP)

## Contexto

Cola (`next`/`prev`) complica estado, IPC y tests.

## Decisión

MVP **sin cola**: una sola pista. Un `play` nuevo **reemplaza** la actual (`loadfile replace`). Cola se pospone a post-MVP (ahora cubierta por 0008).

## Consecuencias

- Superficie de comandos más chica (`search`, `play`, `pause`, `resume`, `stop`, `volume`, `status`, `now`).
- No hay auto-avance a “siguiente”.
