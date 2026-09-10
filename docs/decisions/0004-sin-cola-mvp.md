# 0004 — Sin cola en el MVP

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

Cola (`next`/`prev`) complica estado, IPC y tests.

## Decisión

MVP **sin cola**: una sola pista. Un `play` nuevo **reemplaza** la actual (`loadfile replace`). Cola se pospone a la fase TUI / post-MVP.

## Consecuencias

- Superficie de comandos más chica (`search`, `play`, `pause`, `resume`, `stop`, `volume`, `status`, `now`).
- No hay auto-avance a “siguiente”.
