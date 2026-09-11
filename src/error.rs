use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, YtcliError>;

#[derive(Debug, Error)]
pub enum YtcliError {
    #[error("no se encontró `{0}` en PATH; instálalo e inténtalo de nuevo")]
    MissingBinary(String),

    #[error("sin resultados para la búsqueda")]
    NoSearchResults,

    #[error("no hay resultados de search previos; ejecuta `ytcli search` primero")]
    NoLastSearch,

    #[error("índice {index} fuera de rango (1..{len})")]
    IndexOutOfRange { index: usize, len: usize },

    #[error("no hay reproducción activa")]
    NotPlaying,

    #[error("no hay siguiente pista en la cola")]
    NoNext,

    #[error("no hay pista anterior en la cola")]
    NoPrevious,

    #[error("volumen inválido: {0} (debe estar entre 0 y 100)")]
    InvalidVolume(u8),

    #[error("falló yt-dlp: {0}")]
    YtDlp(String),

    #[error("falló mpv/IPC ({path}): {detail}")]
    MpvIpc { path: PathBuf, detail: String },

    #[error("no se pudo escribir el estado en {path}: {detail}")]
    StateIo { path: PathBuf, detail: String },

    #[error("falló Spotify: {0}")]
    Spotify(String),

    #[error(
        "faltan credenciales de Spotify; exporta SPOTIFY_CLIENT_ID y SPOTIFY_CLIENT_SECRET (app en https://developer.spotify.com/dashboard)"
    )]
    MissingSpotifyCredentials,

    #[error("JSON inválido: {0}")]
    Json(String),

    #[error("nombre de playlist inválido: {0}")]
    InvalidPlaylistName(String),

    #[error("no existe la playlist `{0}`; prueba `ytcli playlists`")]
    PlaylistNotFound(String),

    #[error("ya existe la playlist `{0}`")]
    PlaylistExists(String),

    #[error("confirma el borrado con `ytcli playlist-rm {0} --yes`")]
    PlaylistRmNeedsYes(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn missing_binary_display() {
        let err = YtcliError::MissingBinary("yt-dlp".into());
        assert_eq!(
            err.to_string(),
            "no se encontró `yt-dlp` en PATH; instálalo e inténtalo de nuevo"
        );
    }

    #[test]
    fn no_search_results_display() {
        let err = YtcliError::NoSearchResults;
        assert_eq!(err.to_string(), "sin resultados para la búsqueda");
    }

    #[test]
    fn no_last_search_display() {
        let err = YtcliError::NoLastSearch;
        assert_eq!(
            err.to_string(),
            "no hay resultados de search previos; ejecuta `ytcli search` primero"
        );
    }

    #[test]
    fn index_out_of_range_display() {
        let err = YtcliError::IndexOutOfRange { index: 5, len: 3 };
        assert_eq!(err.to_string(), "índice 5 fuera de rango (1..3)");
    }

    #[test]
    fn not_playing_display() {
        let err = YtcliError::NotPlaying;
        assert_eq!(err.to_string(), "no hay reproducción activa");
    }

    #[test]
    fn no_next_display() {
        let err = YtcliError::NoNext;
        assert_eq!(err.to_string(), "no hay siguiente pista en la cola");
    }

    #[test]
    fn no_previous_display() {
        let err = YtcliError::NoPrevious;
        assert_eq!(err.to_string(), "no hay pista anterior en la cola");
    }

    #[test]
    fn invalid_volume_display() {
        let err = YtcliError::InvalidVolume(101);
        assert_eq!(
            err.to_string(),
            "volumen inválido: 101 (debe estar entre 0 y 100)"
        );
    }

    #[test]
    fn yt_dlp_display() {
        let err = YtcliError::YtDlp("exit 1".into());
        assert_eq!(err.to_string(), "falló yt-dlp: exit 1");
    }

    #[test]
    fn mpv_ipc_display() {
        let err = YtcliError::MpvIpc {
            path: PathBuf::from("/tmp/mpv.sock"),
            detail: "connection refused".into(),
        };
        assert_eq!(
            err.to_string(),
            "falló mpv/IPC (/tmp/mpv.sock): connection refused"
        );
    }

    #[test]
    fn state_io_display() {
        let err = YtcliError::StateIo {
            path: PathBuf::from("/tmp/state.json"),
            detail: "permission denied".into(),
        };
        assert_eq!(
            err.to_string(),
            "no se pudo escribir el estado en /tmp/state.json: permission denied"
        );
    }

    #[test]
    fn json_display() {
        let err = YtcliError::Json("unexpected EOF".into());
        assert_eq!(err.to_string(), "JSON inválido: unexpected EOF");
    }

    #[test]
    fn spotify_display() {
        let err = YtcliError::Spotify("boom".into());
        assert_eq!(err.to_string(), "falló Spotify: boom");
    }

    #[test]
    fn missing_spotify_credentials_display() {
        let err = YtcliError::MissingSpotifyCredentials;
        assert!(err.to_string().contains("SPOTIFY_CLIENT_ID"));
    }

    #[test]
    fn io_display() {
        let err = YtcliError::Io(io::Error::new(io::ErrorKind::NotFound, "file missing"));
        assert!(err.to_string().contains("file missing"));
    }
}
