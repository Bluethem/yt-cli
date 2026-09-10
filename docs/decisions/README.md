# Decisiones de diseño — ytcli

Registro de decisiones (ADR ligero). Specs:

- MVP: [`../superpowers/specs/2026-09-10-ytcli-design.md`](../superpowers/specs/2026-09-10-ytcli-design.md)
- Cola: [`../superpowers/specs/2026-09-10-ytcli-queue-design.md`](../superpowers/specs/2026-09-10-ytcli-queue-design.md)

| ID | Decisión | Elección |
|----|----------|----------|
| [0001](0001-fuente-youtube.md) | Fuente de música | YouTube streaming |
| [0002](0002-stack-rust-ytdlp-mpv.md) | Stack de reproducción | Rust + yt-dlp + mpv IPC |
| [0003](0003-ux-search-y-play.md) | UX de selección | `play` query + `search`/`play N` |
| [0004](0004-sin-cola-mvp.md) | Cola en MVP | No (histórico; superseded por 0008) |
| [0005](0005-daemon-background.md) | Modelo de proceso | mpv daemon + IPC |
| [0006](0006-arquitectura-orquestadora.md) | Arquitectura | CLI orquestadora (enf. 1) |
| [0007](0007-roadmap-cli-luego-tui.md) | Roadmap | CLI → cola → TUI / sesión |
| [0008](0008-cola-watch-next-prev.md) | Cola + watch | state + URL al vuelo + next/prev |
