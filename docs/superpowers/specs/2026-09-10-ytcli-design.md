# ytcli — Design Spec (CLI de música vía YouTube)

**Fecha:** 2026-09-10  
**Estado:** Aprobado  
**Nombre de trabajo del binario:** `ytcli` (renombrable sin cambiar la arquitectura)

## Resumen

CLI en Rust para escuchar música en streaming desde YouTube, con bajo consumo de recursos. El MVP es una CLI básica con reproducción en background; una TUI interactiva llega en una fase posterior reutilizando los mismos módulos de dominio.

La CLI **no decodifica audio**. Orquesta binarios externos:

- **yt-dlp** — búsqueda y resolución de URL de stream de audio
- **mpv** — reproducción y control vía IPC

## Metas

- Reproducir audio de YouTube sin archivos locales
- Pocos recursos (CPU/RAM): sin decoder propio en Rust en el MVP
- Comandos de control útiles desde otra invocación de la CLI (`pause`, `volume`, etc.)
- Base clara para una TUI futura

## No-metas (MVP)

- Cola de reproducción (`next`/`prev`/playlist)
- TUI / ratatui
- Caché de audio en disco
- Autenticación YouTube / cookies (salvo que yt-dlp del usuario ya las use)
- CI que pegue a YouTube real
- Reproducción multi-instancia (un solo mpv por usuario)

## Decisiones de producto (cerradas)

| Decisión | Elección |
|----------|----------|
| Fuente | YouTube (streaming) |
| Lenguaje | Rust |
| Player | yt-dlp + mpv (subprocess + IPC) |
| UX de selección | `play "query"` (1.er resultado) **y** `search` + `play N` |
| Cola | Fuera del MVP (una pista a la vez; `play` reemplaza) |
| Modelo de proceso | Daemon: mpv en background; CLI vuelve al prompt |
| Enfoque arquitectónico | CLI orquestadora + mpv IPC (no rodio, no wrappers ciegos) |

Registro ampliado: [`docs/decisions/`](../../decisions/).

## Arquitectura

```
usuario
  └─ ytcli (clap)
       ├─ youtube  → yt-dlp (search / get-url)
       ├─ player   → mpv (--input-ipc-server)
       └─ state    → ~/.cache/ytcli/state.json
```

- Un solo proceso **mpv** en background por usuario.
- Control (`pause`, `resume`, `stop`, `volume`) por socket IPC de mpv.
- `search` / `play` usan yt-dlp; `play` pide audio-only y carga la URL en mpv.
- La TUI futura consume los mismos módulos `youtube`, `player` y `state`.

### Enfoques descartados

1. **Daemon Rust + rodio** — más código, más CPU/RAM, peor encaje con “pocos recursos”.
2. **Solo wrappers shell sin IPC** — no soporta bien `pause`/`volume` como comandos separados.

## Componentes

| Módulo | Responsabilidad |
|--------|-----------------|
| `cli` | Parseo **clap**: `search`, `play`, `pause`, `resume`, `stop`, `volume`, `status`, `now` |
| `youtube` | Invoca yt-dlp: búsqueda (JSON) y URL de audio |
| `player` | Arranca/controla mpv vía IPC (`loadfile`, `pause`, `volume`, `quit`) |
| `state` | Persiste pid, ruta del socket, track actual y último search |
| `error` | Errores claros: binarios faltantes, sin resultados, mpv/IPC caído |

### Comportamiento de comandos

- `search "q"` — lista numerada (título, canal, duración); guarda resultados en `state.last_search`
- `play <target>` — un solo argumento:
  - si `target` es un entero **≥ 1** → índice 1-based de `state.last_search`
  - si no → query de búsqueda; se toma el **primer** resultado
- `play` nuevo — **reemplaza** la pista actual (`loadfile … replace`)
- `pause` / `resume` / `stop` / `volume <0-100>` — IPC
- `status` / `now` — lee `state` + comprueba si mpv responde

Índices de search/`play N` son **1-based** (el primero de la lista es `1`).

### Dependencias Rust (MVP)

- `clap` — CLI
- `serde` / `serde_json` — state y salida yt-dlp
- `thiserror` — errores
- `dirs` — ruta de cache

Sin `rodio` / `cpal` en el MVP.

### Dependencias de runtime

- `yt-dlp` en `PATH`
- `mpv` en `PATH`

Comprobar al inicio de los comandos que las necesitan.

## Flujo de datos

### `search "lo-fi beats"`

1. CLI invoca yt-dlp en modo búsqueda (salida JSON).
2. Parsea N resultados (por defecto 10).
3. Imprime lista numerada.
4. Guarda en `state.last_search`.

### `play "lo-fi beats"` o `play 2`

1. Resuelve el track (primer hit o índice del último search).
2. `yt-dlp -f bestaudio -g <url>` → URL directa de stream.
3. Si no hay mpv vivo: arrancar con `--no-video --input-ipc-server=<socket>`.
4. IPC: `loadfile <url> replace`.
5. Actualiza `state` (pid, socket, `now_playing`).

### `pause` / `resume` / `volume 40` / `stop`

1. Lee socket desde `state`.
2. Si el proceso no responde → error (“no hay reproducción activa”) y limpia state si aplica.
3. Envía comando IPC; `stop` hace `quit` y borra `now_playing`.

## Estado en disco

Ruta: `~/.cache/ytcli/state.json` (o equivalente vía `dirs::cache_dir()`).

Campos previstos:

```json
{
  "mpv_pid": 12345,
  "ipc_socket": "/home/user/.cache/ytcli/mpv.sock",
  "now_playing": {
    "id": "dQw4w9WgXcQ",
    "title": "Example",
    "uploader": "Channel",
    "webpage_url": "https://www.youtube.com/watch?v=...",
    "duration_secs": 212
  },
  "last_search": [
    {
      "id": "...",
      "title": "...",
      "uploader": "...",
      "webpage_url": "...",
      "duration_secs": 180
    }
  ]
}
```

## Manejo de errores

| Situación | Comportamiento |
|-----------|----------------|
| Falta `yt-dlp` o `mpv` | Error claro + hint de instalación; exit ≠ 0 |
| Búsqueda sin resultados | Mensaje amigable; no tocar reproducción actual |
| `play N` sin search / N inválido | Error; no llamar a yt-dlp ni mpv |
| URL de stream falla | Error; no dejar mpv a medias si aún no cargó |
| IPC muerto / pid zombie | Detectar, limpiar `state`, pedir `play` de nuevo |
| mpv ya corriendo + `play` nuevo | `loadfile replace` (una sola instancia) |
| Socket no escribible | Error con ruta del socket |

Principio: fallar ruidoso y temprano. Flag `--verbose` puede añadirse después para reenviar más stderr de yt-dlp/mpv.

## Testing (MVP)

Poco E2E real contra YouTube (frágil/red); mucho unitario con fakes.

| Capa | Qué probar |
|------|------------|
| `youtube` | Parseo JSON (fixtures); argumentos del comando construidos bien |
| `player` | Mensajes IPC; detección de “mpv vivo” con fake de socket |
| `state` | Read/write/roundtrip; limpieza tras `stop` |
| `cli` | Parsing clap + routing con mocks |
| Manual | search → play N → pause → volume → stop; play query; binarios ausentes |

Fuera de alcance: CI contra YouTube real; tests de TUI.

## Layout de crates / archivos (previsto)

```
src/
  main.rs
  cli.rs
  youtube.rs
  player.rs
  state.rs
  error.rs
tests/
  fixtures/   # JSON de ejemplo de yt-dlp
docs/
  decisions/
  superpowers/
    specs/
    plans/    # plan de implementación (siguiente paso)
```

## Fase 2 (fuera de este spec de implementación inmediata)

- TUI con `ratatui` + `crossterm`
- Cola (`queue`, `next`, `prev`)
- Mismos backends `youtube` / `player` / `state`

## Riesgos y notas

- YouTube / yt-dlp pueden romper extractores; mitigación: pin o documentar versión mínima de yt-dlp.
- ToS de YouTube: uso personal / experimental; no se pretende redistribuir audio.
- mpv debe estar instalado en la máquina del usuario (en este entorno de diseño, verificar en setup).
