# ytcli

CLI ligera para buscar y escuchar audio de YouTube desde la terminal, usando **yt-dlp** para búsqueda/resolución de URLs y **mpv** como reproductor en segundo plano (IPC).

## Requisitos

| Dependencia | Uso |
|-------------|-----|
| **Rust** (toolchain estable) | Compilar e instalar |
| **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** | Búsqueda y extracción de URL de audio |
| **[mpv](https://mpv.io/)** | Reproducción vía socket IPC |

Comprueba que los binarios estén en el `PATH`:

```bash
which yt-dlp mpv
```

En Arch Linux:

```bash
sudo pacman -S yt-dlp mpv
```

## Instalación

Desde el directorio del proyecto:

```bash
cargo install --path .
```

O ejecutar sin instalar:

```bash
cargo run -- <comando> [args]
```

## Uso

### Buscar

```bash
ytcli search "lofi hip hop" -n 5
```

Muestra resultados numerados (1-based). Los guarda en `~/.cache/ytcli/state.json` para `play N`.

### Reproducir

Por índice del último `search`:

```bash
ytcli play 1
```

Por consulta directa (reproduce el primer resultado):

```bash
ytcli play "lofi hip hop"
```

**Nota MVP:** solo hay **un** proceso mpv. Cada `play` **reemplaza** la pista actual; no hay cola ni historial de reproducción.

### Control de reproducción

```bash
ytcli pause
ytcli resume
ytcli volume 50    # 0–100
ytcli stop
```

### Estado

```bash
ytcli now      # pista actual
ytcli status   # ¿mpv activo?
```

## Ejemplo de flujo

```bash
ytcli search "jazz piano" -n 5
ytcli play 2
ytcli now
ytcli pause
ytcli resume
ytcli volume 40
ytcli stop
```

## Datos locales

- Estado: `~/.cache/ytcli/state.json` (última búsqueda, pista actual, PID de mpv)
- Socket IPC: `~/.cache/ytcli/mpv.sock`

## Desarrollo

```bash
cargo test
```

Los tests unitarios no requieren `mpv` ni red; usan fixtures y mocks donde aplica.

## Licencia

MIT
