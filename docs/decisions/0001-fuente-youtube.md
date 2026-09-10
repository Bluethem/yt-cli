# 0001 — Fuente: YouTube streaming

**Fecha:** 2026-09-10  
**Estado:** Aceptada

## Contexto

No hay biblioteca local. Se necesita catálogo amplio sin gestionar archivos.

## Decisión

Usar **YouTube** como única fuente del MVP, vía **yt-dlp** (búsqueda + URL de audio).

## Consecuencias

- Dependencia de extractores de yt-dlp (pueden romper).
- No hay API oficial estable de “YouTube Music” en este diseño.
- Spotify / radios quedan fuera del MVP (el trait/backend futuro puede ampliarse).
