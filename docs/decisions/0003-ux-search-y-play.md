# 0003 — UX: play directo y search + índice

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

El usuario a veces sabe qué quiere; a veces necesita elegir entre resultados.

## Decisión

Soportar **ambos**:

- `ytcli play "query"` → reproduce el primer resultado.
- `ytcli search "query"` → lista numerada; `ytcli play N` → elige el N del último search.

## Consecuencias

- `state.last_search` es necesario para `play N`.
- `play` con string vs entero debe distinguirse en el parsing de clap (subcomandos o value parser).
