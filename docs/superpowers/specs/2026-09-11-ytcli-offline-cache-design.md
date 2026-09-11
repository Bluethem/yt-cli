# ytcli — Design Spec: Offline cache (Opus) + stream/file resolve

**Fecha:** 2026-09-11  
**Estado:** Aprobado  
**Extiende:** playlists locales (`2026-09-11-ytcli-local-playlists-design.md`), cola y reproducción stream  
**Runtime nuevo:** `ffmpeg` en PATH (además de yt-dlp + mpv)

## Resumen

Conviven **streaming** (actual) y **archivo local**. Al reproducir se prefiere el archivo en cache si existe y es válido; si no, stream. Descarga: yt-dlp `bestaudio` → ffmpeg → **Opus 160 kbps VBR**. Un archivo por `Track.id`, compartido entre playlists. La cola marca local vs stream con prefijo + color.

## Metas

- Descargar con buena calidad y poco peso (Opus 160).
- Cache indexado por id YouTube (una copia, N playlists).
- Play/watch automático: local si hay, si no stream.
- UI: `[↓]`+verde local, `[~]` stream; `NO_COLOR` / `--no-color`.
- Comando `download` + flags `--download` en `save` / `playlist --save`.

## No-metas

- MP3 como default (posible flag futuro).
- Modo global obligatorio stream|file.
- Copiar audio dentro de cada JSON de playlist.
- TUI / instalador.
- Re-encode batch de toda la librería en un solo comando (salvo lo cubierto por `download <playlist>`).

## Decisiones cerradas

| Tema | Elección |
|------|----------|
| Playback | Preferir archivo si existe y es válido; si no → stream (B) |
| Cuándo descargar | `download` + `--download` en save/playlist (C) |
| Códec default | Opus 160 kbps VBR vía ffmpeg `libopus` |
| Fuente yt-dlp | `-f bestaudio` (temp) luego convert |
| Storage | `~/.cache/ytcli/audio/<id>.opus` + `index.json` (A) |
| Playlists JSON | Sin `local_path`; solo metadata `Track` |
| UI | Prefijo + ANSI; apagable (C) |
| Deps crates | Sin decoder propio; orquestar binarios |

## Comandos

| Comando | Comportamiento |
|---------|----------------|
| `ytcli download` | Pista actual → cache |
| `ytcli download <playlist>` | Todas las pistas de playlist local; skip si ya cached |
| `ytcli save [--download]` | Append (+ download opcional) |
| `ytcli playlist URL --save [name] [--download] [--play]` | Persistencia + download opcional + play |
| `ytcli --no-color <cmd>` | Sin ANSI (también `NO_COLOR=1`) |

`ytcli cache` (listar/limpiar) queda **opcional / fase 2**; no bloquea v1.

## Index

`~/.cache/ytcli/audio/index.json`:

```json
{
  "<youtube_id>": {
    "path": "<youtube_id>.opus",
    "title": "…",
    "uploader": "…",
    "bitrate_kbps": 160,
    "codec": "opus",
    "bytes": 1234567,
    "downloaded_at": "…"
  }
}
```

- Escritura atómica del index y del `.opus` (temp + rename).
- Entrada al index **solo** tras encode exitoso.

## Resolve de reproducción

Función única usada por `play`, `watch`, `next`/`prev` load, `open --play`, `playlist --play`:

1. Si `cache.get(id)` y el archivo existe → `mpv loadfile` path absoluto.  
2. Si falta, size 0, o loadfile local falla → stream `resolve_audio_url` (+ aviso breve si falló local).

## Display

En `queue` y `show` (y status si aplica):

- Local: prefijo `[↓]` + verde (ANSI).  
- Stream: prefijo `[~]` + estilo default.  
- Se conservan `>` (actual) y `🔁` (loop).

## Arquitectura

```
ytcli
  ├─ cache     → index + paths bajo cache_dir/audio
  ├─ download  → yt-dlp bestaudio + ffmpeg opus 160
  ├─ display   → markers + color / NO_COLOR
  ├─ commands  → download, flags; resolve antes de loadfile
  ├─ watch     → mismo resolve
  └─ playlists → sin cambios de schema (solo ids)
```

## Errores

| Caso | Comportamiento |
|------|----------------|
| Sin ffmpeg / yt-dlp | Error claro en español |
| Encode falla | No tocar index; limpiar temps |
| Archivo local ilegible en play | Fallback stream + aviso |
| `download` sin pista / playlist vacía | Error / mensaje vacío |

## Tests

- cache put/get; skip download si ya indexed + file exists.  
- resolve: file vs stream (tmpdir, sin red).  
- display con/sin `NO_COLOR`.  
- CLI parse: `download`, `--download`, `--no-color`.  
- Download real: no en unit tests (o mock de Command).

## Criterios de éxito

- Misma canción en 2 playlists → un solo `.opus`.  
- Tras `download`, `queue` muestra `[↓]` y reproduce sin red (si el archivo está).  
- Sin cache, comportamiento stream idéntico al actual.  
- `ffmpeg` documentado en README.
