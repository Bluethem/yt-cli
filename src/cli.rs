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
}
