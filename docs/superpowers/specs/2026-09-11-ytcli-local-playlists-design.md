# ytcli — Design Spec: Loop en queue + playlists locales (JSON)

**Fecha:** 2026-09-11  
**Estado:** Aprobado  
**Extiende:** [`2026-09-10-ytcli-polish-design.md`](./2026-09-10-ytcli-polish-design.md), cola y Spotify specs  
**Roadmap:** tras Spotify mirror; **antes** de TUI (ADR 0007). Descarga MP3 = fase posterior.

## Resumen

1. **`queue` muestra loop** — si `loop_current`, la pista actual se marca (p. ej. `>` + `🔁`).
2. **Playlists locales en JSON** — metadata only; reproducción = stream (yt-dlp + mpv) como la cola.
3. **Guardado** — `save` / `save <nombre>` y `playlist <url> --save [nombre]`.
4. **Abrir / listar / borrar** playlists locales desde CLI.

## Metas

- Ver de un vistazo si la canción actual está en loop al listar la cola.
- Guardar pistas y playlists de YouTube en disco sin Premium ni scrapeo.
- Reabrir una playlist local y encolarla (stream al vuelo).
- Default explícito: playlist `liked` cuando no se nombra destino.

## No-metas

- Descarga offline / MP3 / elección de bitrate (fase siguiente; yt-dlp + re-encode).
- TUI, instalador, elegir “solo TUI”.
- SQLite u otra DB.
- Sync con Spotify hacia playlist local (sí se puede `save` si la pista ya está en cola vía mirror).
- Loop de playlist completa / repeat-all.
- Edición interactiva de orden dentro de la playlist (más allá de re-import / open + shuffle de cola).

## Decisiones cerradas

| Tema | Elección |
|------|----------|
| Marker loop en `queue` | Pista actual: `>` y, si `loop_current`, indicador `🔁` |
| Storage | Un JSON por playlist bajo XDG data |
| Path | `dirs::data_dir()/ytcli/playlists/<slug>.json` |
| Default playlist | `liked` |
| `ytcli save` | Append pista **actual** → `liked` (dedup por `id`) |
| `ytcli save <nombre>` | Append pista actual → esa lista (crea si no existe) |
| `playlist <url> --save` | Persiste import YT; sin nombre → append+dedup a `liked` |
| `playlist <url> --save <nombre>` | **Replace** tracks de esa playlist local + `source` = URL |
| Contenido JSON | Solo metadata de `Track` (+ timestamps / source) |
| Reproducción | Stream; no archivos de audio en esta fase |
| `open <nombre>` | Carga en cola; sin autoplay. `--play` → índice 0 |
| Dedup | Por `id` de YouTube al append |
| TUI / instalador | Fuera; CLI siempre será base cuando exista TUI |
| MP3 | Documentado como futuro: yt-dlp audio + calidad ≈ códec/bitrate |

## Comandos

| Comando | Comportamiento |
|---------|----------------|
| `ytcli queue` | Lista; actual `>`; si loop, `🔁` en esa línea |
| `ytcli save` | Actual → `liked` |
| `ytcli save <nombre>` | Actual → `<nombre>` |
| `ytcli playlist <url> [--save [nombre]] [--play] [--limit N]` | Como hoy + persistencia opcional |
| `ytcli playlists` | Nombre + cantidad de tracks |
| `ytcli open <nombre> [--play]` | Cola = tracks de la playlist; limpia `loop_current` |
| `ytcli playlist-rm <nombre> --yes` | Borra JSON; **requiere** `--yes` (CLI no interactiva) |

Nombre inválido (vacío, `..`, separadores de path): rechazo.

## Formato JSON

```json
{
  "name": "liked",
  "created_at": "2026-09-11T12:00:00Z",
  "updated_at": "2026-09-11T12:05:00Z",
  "source": null,
  "tracks": [
    {
      "id": "dQw4w9WgXcQ",
      "title": "…",
      "uploader": "…",
      "webpage_url": "https://www.youtube.com/watch?v=…",
      "duration_secs": 212
    }
  ]
}
```

- `source`: URL de YouTube si la lista se creó/reemplazó con `playlist --save <nombre>`; `null` en `liked` acumulativo o listas solo con `save`.
- Campos de track alineados con `Track` en `state` (mismo shape serializable).
- Escritura atómica (temp + rename), mismo criterio que `state.json` si es práctico.

## Arquitectura

```
ytcli
  ├─ playlists  → path, slug, load/save, list, delete, append_dedup, replace_from_import
  ├─ commands   → save, playlists, open, playlist-rm; queue marker; playlist --save
  ├─ cli        → nuevos subcomandos / flags
  ├─ state      → cola de sesión sin mezclar librería
  └─ (futuro)   → download mp3 vía yt-dlp
```

- Cola (`state.json` en cache) = sesión.
- Playlists (`data_dir`) = librería persistente.
- `open` copia tracks a la cola; no “vincula” en vivo el JSON (cambios posteriores al JSON no alteran la cola hasta otro `open`).

## Errores

| Caso | Resultado |
|------|-----------|
| `save` sin pista actual | Error: no hay reproducción / índice actual |
| `open` / `playlist-rm` inexistente | Not found + hint `ytcli playlists` |
| Nombre inválido | Error de validación |
| I/O / JSON corrupto | `YtcliError` con path |

## Tests

- Slug / reject path traversal.
- Append + dedup; replace en `--save <nombre>`.
- Default `liked` vs nombre explícito.
- `queue` output helper: con/sin `loop_current`.
- Sin red en unit tests de `playlists`.

## Fase posterior (no implementar ahora)

- `ytcli download` / `--download`: yt-dlp → audio; opción MP3 (re-encode) vs bestaudio.
- Calidad: códec + bitrate (p. ej. 128/192/320 kbps para MP3); YouTube ya entrega lossy.
- TUI e instalador: CLI + TUI opcional, no mutuamente excluyentes.

## Criterios de éxito

- `queue` refleja loop sin consultar otro comando.
- Tras `save` / `playlist --save`, `playlists` y `open` restauran la lista y se puede reproducir en stream.
- Sin dependencias nuevas de crates de audio ni disco de MP3 en esta fase.
