use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "ytcli",
    version,
    about = "Escucha YouTube desde la terminal (yt-dlp + mpv)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Busca videos/canciones en YouTube
    Search {
        /// Texto de búsqueda
        query: String,
        /// Máximo de resultados
        #[arg(
            short = 'n',
            long,
            default_value_t = 10,
            value_parser = parse_search_limit
        )]
        limit: usize,
    },
    /// Reproduce: query (1.er resultado) o índice 1-based del último search
    Play {
        target: String,
    },
    /// Encola sin interrumpir la reproducción actual
    Add {
        target: String,
    },
    /// Muestra la cola de reproducción
    #[command(visible_alias = "list")]
    Queue,
    /// Pasa a la siguiente pista en cola
    Next,
    /// Pista anterior o reinicio si llevas poco tiempo
    Prev {
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Deja en cola solo la pista actual
    Clear,
    /// Mezcla solo las pistas pendientes (después de la actual)
    Shuffle,
    /// Importa una playlist de YouTube
    Playlist {
        url: String,
        /// Reemplaza la cola y reproduce desde el primero
        #[arg(long)]
        play: bool,
        /// Máximo de pistas a importar
        #[arg(
            short = 'n',
            long,
            default_value_t = 100,
            value_parser = parse_search_limit
        )]
        limit: usize,
        /// Guardar local: `--save` → liked (append); `--save NOMBRE` → replace.
        /// Compatible con `--play` (p. ej. `--save favorites --play`).
        #[arg(long, num_args = 0..=1, value_name = "NOMBRE", allow_hyphen_values = false)]
        save: Option<Option<String>>,
    },
    /// Guarda la pista actual en una playlist local (default: liked)
    Save {
        /// Destino (default liked)
        name: Option<String>,
    },
    /// Crea una playlist local vacía
    Create {
        /// Nombre de la playlist
        name: String,
    },
    /// Lista playlists locales guardadas
    Playlists,
    /// Muestra las pistas de una playlist local (no toca la cola)
    Show {
        /// Nombre de la playlist
        name: String,
    },
    /// Encola una playlist local al final de la cola actual (o reproduce con --play)
    Open {
        /// Nombre de la playlist
        name: String,
        /// Reemplaza la cola de sesión y reproduce desde el primero
        #[arg(long)]
        play: bool,
    },
    /// Elimina una playlist local
    #[command(name = "playlist-rm")]
    PlaylistRm {
        /// Nombre de la playlist
        name: String,
        /// Confirmar eliminación
        #[arg(long)]
        yes: bool,
    },
    /// Repite la pista actual
    Loop,
    /// Quita el loop de la pista actual
    Unloop,
    /// Bucle interno de auto-avance (avanzado)
    Watch,
    Pause,
    Resume,
    Stop,
    /// Volumen 0-100
    Volume {
        #[arg(value_parser = parse_volume)]
        level: u8,
    },
    /// ¿Hay mpv activo?
    Status,
    /// Pista actual
    Now,
}

fn parse_search_limit(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(limit) if limit >= 1 => Ok(limit),
        _ => Err("el límite debe ser un entero mayor o igual a 1".into()),
    }
}

fn parse_volume(value: &str) -> Result<u8, String> {
    match value.parse::<u8>() {
        Ok(level) if level <= 100 => Ok(level),
        _ => Err("el volumen debe ser un entero entre 0 y 100".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_play_index() {
        let cli = Cli::try_parse_from(["ytcli", "play", "3"]).unwrap();
        match cli.command {
            Commands::Play { target } => assert_eq!(target, "3"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_volume() {
        let cli = Cli::try_parse_from(["ytcli", "volume", "40"]).unwrap();
        match cli.command {
            Commands::Volume { level } => assert_eq!(level, 40),
            _ => panic!(),
        }
    }

    #[test]
    fn reject_invalid_ranges() {
        assert!(Cli::try_parse_from(["ytcli", "volume", "101"]).is_err());
        assert!(Cli::try_parse_from(["ytcli", "search", "lofi", "-n", "0"]).is_err());
    }

    #[test]
    fn parse_add() {
        let cli = Cli::try_parse_from(["ytcli", "add", "2"]).unwrap();
        match cli.command {
            Commands::Add { target } => assert_eq!(target, "2"),
            _ => panic!("expected Add"),
        }
    }

    #[test]
    fn parse_prev_force() {
        let cli = Cli::try_parse_from(["ytcli", "prev", "-f"]).unwrap();
        match cli.command {
            Commands::Prev { force: true } => {}
            _ => panic!("expected Prev with force"),
        }

        let cli = Cli::try_parse_from(["ytcli", "prev", "--force"]).unwrap();
        match cli.command {
            Commands::Prev { force: true } => {}
            _ => panic!("expected Prev with --force"),
        }
    }

    #[test]
    fn parse_queue() {
        let cli = Cli::try_parse_from(["ytcli", "queue"]).unwrap();
        assert!(matches!(cli.command, Commands::Queue));
    }

    #[test]
    fn parse_list_alias() {
        let cli = Cli::try_parse_from(["ytcli", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::Queue));
    }

    #[test]
    fn parse_playlist_play_and_limit() {
        let cli = Cli::try_parse_from([
            "ytcli",
            "playlist",
            "https://youtube.com/playlist?list=PLx",
            "--play",
            "-n",
            "25",
        ])
        .unwrap();
        match cli.command {
            Commands::Playlist {
                url,
                play,
                limit,
                save,
            } => {
                assert!(url.contains("list=PLx"));
                assert!(play);
                assert_eq!(limit, 25);
                assert!(save.is_none());
            }
            _ => panic!("expected Playlist"),
        }
    }

    #[test]
    fn parse_save_default_and_named() {
        let cli = Cli::try_parse_from(["ytcli", "save"]).unwrap();
        match cli.command {
            Commands::Save { name: None } => {}
            _ => panic!(),
        }
        let cli = Cli::try_parse_from(["ytcli", "save", "rock"]).unwrap();
        match cli.command {
            Commands::Save { name: Some(n) } => assert_eq!(n, "rock"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_playlist_save_optional_name() {
        let cli = Cli::try_parse_from(["ytcli", "playlist", "https://x", "--save"]).unwrap();
        match cli.command {
            Commands::Playlist { save: Some(None), .. } => {}
            other => panic!("{other:?}"),
        }
        let cli = Cli::try_parse_from(["ytcli", "playlist", "https://x", "--save", "rock"]).unwrap();
        match cli.command {
            Commands::Playlist { save: Some(Some(n)), .. } => assert_eq!(n, "rock"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_playlist_save_and_play_together() {
        let cli = Cli::try_parse_from([
            "ytcli",
            "playlist",
            "https://x",
            "--save",
            "favorites",
            "--play",
        ])
        .unwrap();
        match cli.command {
            Commands::Playlist {
                save: Some(Some(name)),
                play: true,
                ..
            } => assert_eq!(name, "favorites"),
            other => panic!("{other:?}"),
        }

        // Bare --save must not swallow --play as the playlist name.
        let cli = Cli::try_parse_from(["ytcli", "playlist", "https://x", "--save", "--play"])
            .unwrap();
        match cli.command {
            Commands::Playlist {
                save: Some(None),
                play: true,
                ..
            } => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_create() {
        let cli = Cli::try_parse_from(["ytcli", "create", "chill"]).unwrap();
        match cli.command {
            Commands::Create { name } => assert_eq!(name, "chill"),
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn parse_show() {
        let cli = Cli::try_parse_from(["ytcli", "show", "liked"]).unwrap();
        match cli.command {
            Commands::Show { name } => assert_eq!(name, "liked"),
            _ => panic!("expected Show"),
        }
    }

    #[test]
    fn parse_playlist_rm_requires_yes_flag_present_in_argv() {
        let cli = Cli::try_parse_from(["ytcli", "playlist-rm", "rock", "--yes"]).unwrap();
        match cli.command {
            Commands::PlaylistRm { name, yes: true } => assert_eq!(name, "rock"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_shuffle_loop_unloop() {
        assert!(matches!(
            Cli::try_parse_from(["ytcli", "shuffle"]).unwrap().command,
            Commands::Shuffle
        ));
        assert!(matches!(
            Cli::try_parse_from(["ytcli", "loop"]).unwrap().command,
            Commands::Loop
        ));
        assert!(matches!(
            Cli::try_parse_from(["ytcli", "unloop"]).unwrap().command,
            Commands::Unloop
        ));
    }
}
