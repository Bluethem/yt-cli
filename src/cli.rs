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
            Commands::Playlist { url, play, limit } => {
                assert!(url.contains("list=PLx"));
                assert!(play);
                assert_eq!(limit, 25);
            }
            _ => panic!("expected Playlist"),
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
