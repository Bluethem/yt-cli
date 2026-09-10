//! Spotify Client Credentials → metadata → YouTube mirror via yt-dlp.

use crate::state::Track;
use crate::youtube::YtDlp;
use crate::{Result, YtcliError};
use serde::Deserialize;
use std::env;

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API_BASE: &str = "https://api.spotify.com/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotifyKind {
    Playlist,
    Album,
    Track,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyRef {
    pub kind: SpotifyKind,
    pub id: String,
}

pub fn is_spotify_url(input: &str) -> bool {
    parse_spotify_ref(input).is_ok()
}

pub fn parse_spotify_ref(input: &str) -> Result<SpotifyRef> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("spotify:") {
        let mut parts = rest.split(':');
        let kind = parts.next().ok_or_else(|| invalid_spotify_url(input))?;
        let id = parts.next().ok_or_else(|| invalid_spotify_url(input))?;
        if id.is_empty() || parts.next().is_some() {
            return Err(invalid_spotify_url(input));
        }
        let kind = match kind {
            "playlist" => SpotifyKind::Playlist,
            "album" => SpotifyKind::Album,
            "track" => SpotifyKind::Track,
            _ => return Err(invalid_spotify_url(input)),
        };
        return Ok(SpotifyRef {
            kind,
            id: id.to_string(),
        });
    }

    let url = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let url = url
        .strip_prefix("open.spotify.com/")
        .ok_or_else(|| invalid_spotify_url(input))?;
    let url = url.split('?').next().unwrap_or(url);
    let mut parts = url.split('/');
    let kind = parts.next().ok_or_else(|| invalid_spotify_url(input))?;
    let id = parts.next().ok_or_else(|| invalid_spotify_url(input))?;
    let id = id.split('?').next().unwrap_or(id);
    if id.is_empty() {
        return Err(invalid_spotify_url(input));
    }
    let kind = match kind {
        "playlist" => SpotifyKind::Playlist,
        "album" => SpotifyKind::Album,
        "track" => SpotifyKind::Track,
        _ => return Err(invalid_spotify_url(input)),
    };
    Ok(SpotifyRef {
        kind,
        id: id.to_string(),
    })
}

fn invalid_spotify_url(input: &str) -> YtcliError {
    YtcliError::Spotify(format!("URL o URI de Spotify no válida: {input}"))
}

pub fn mirror_search_query(title: &str, artists: &[String]) -> String {
    let artist = artists
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let title = title.trim();
    if artist.is_empty() {
        title.to_string()
    } else if title.is_empty() {
        artist
    } else {
        format!("{artist} {title}")
    }
}

struct SpotifyClient {
    http: reqwest::blocking::Client,
    token: String,
}

impl SpotifyClient {
    fn from_env() -> Result<Self> {
        let client_id =
            env::var("SPOTIFY_CLIENT_ID").map_err(|_| YtcliError::MissingSpotifyCredentials)?;
        let client_secret =
            env::var("SPOTIFY_CLIENT_SECRET").map_err(|_| YtcliError::MissingSpotifyCredentials)?;
        if client_id.trim().is_empty() || client_secret.trim().is_empty() {
            return Err(YtcliError::MissingSpotifyCredentials);
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| YtcliError::Spotify(e.to_string()))?;
        let token = fetch_token(&http, &client_id, &client_secret)?;
        Ok(Self { http, token })
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| YtcliError::Spotify(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(YtcliError::Spotify(format!(
                "HTTP {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        response
            .json()
            .map_err(|e| YtcliError::Spotify(e.to_string()))
    }
}

fn fetch_token(
    http: &reqwest::blocking::Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String> {
    let response = http
        .post(TOKEN_URL)
        .basic_auth(client_id, Some(client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .map_err(|e| YtcliError::Spotify(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(YtcliError::Spotify(format!(
            "auth HTTP {status}: {}",
            body.chars().take(200).collect::<String>()
        )));
    }
    let token: TokenResponse = response
        .json()
        .map_err(|e| YtcliError::Spotify(e.to_string()))?;
    Ok(token.access_token)
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct NamedResource {
    name: String,
}

#[derive(Debug, Clone)]
struct MetaTrack {
    title: String,
    artists: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistTracksPage {
    items: Vec<PlaylistItem>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    track: Option<ApiTrack>,
}

#[derive(Debug, Deserialize)]
struct AlbumTracksPage {
    items: Vec<ApiTrack>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiTrack {
    name: Option<String>,
    artists: Option<Vec<ApiArtist>>,
}

#[derive(Debug, Deserialize)]
struct ApiArtist {
    name: String,
}

impl ApiTrack {
    fn into_meta(self) -> Option<MetaTrack> {
        let title = self.name?.trim().to_string();
        if title.is_empty() {
            return None;
        }
        let artists = self
            .artists
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.name)
            .filter(|n| !n.trim().is_empty())
            .collect();
        Some(MetaTrack { title, artists })
    }
}

/// Expand a Spotify URL into mirrored YouTube `Track`s (up to `limit`).
pub fn expand_to_tracks(url: &str, limit: usize) -> Result<(Option<String>, Vec<Track>)> {
    let spotify_ref = parse_spotify_ref(url)?;
    let client = SpotifyClient::from_env()?;
    let (title, metas) = fetch_metas(&client, &spotify_ref)?;
    let metas: Vec<_> = metas.into_iter().take(limit).collect();
    if metas.is_empty() {
        return Err(YtcliError::NoSearchResults);
    }

    let yt = YtDlp::default();
    yt.ensure_bin()?;
    let total = metas.len();
    let mut tracks = Vec::new();
    let mut skipped = 0usize;

    for (i, meta) in metas.iter().enumerate() {
        eprintln!("Resolviendo {}/{total}…", i + 1);
        let query = mirror_search_query(&meta.title, &meta.artists);
        match yt.search(&query, 1) {
            Ok(mut found) => {
                if let Some(mut track) = found.pop() {
                    // Prefer Spotify naming in the queue UI.
                    track.title = meta.title.clone();
                    if !meta.artists.is_empty() {
                        track.uploader = meta.artists.join(", ");
                    }
                    tracks.push(track);
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    if tracks.is_empty() {
        return Err(YtcliError::Spotify(
            "no se pudo resolver ninguna pista a YouTube".into(),
        ));
    }
    if skipped > 0 {
        eprintln!("Advertencia: se omitieron {skipped} pistas sin mirror en YouTube.");
    }
    Ok((title, tracks))
}

fn fetch_metas(
    client: &SpotifyClient,
    spotify_ref: &SpotifyRef,
) -> Result<(Option<String>, Vec<MetaTrack>)> {
    match spotify_ref.kind {
        SpotifyKind::Track => {
            let url = format!("{API_BASE}/tracks/{}", spotify_ref.id);
            let track: ApiTrack = client.get_json(&url)?;
            let meta = track
                .into_meta()
                .ok_or_else(|| YtcliError::Spotify("track sin metadatos".into()))?;
            let title = Some(meta.title.clone());
            Ok((title, vec![meta]))
        }
        SpotifyKind::Album => {
            let album_url = format!("{API_BASE}/albums/{}", spotify_ref.id);
            let album: NamedResource = client.get_json(&album_url)?;
            let mut page_url = Some(format!(
                "{API_BASE}/albums/{}/tracks?limit=50",
                spotify_ref.id
            ));
            let mut metas = Vec::new();
            while let Some(url) = page_url {
                let page: AlbumTracksPage = client.get_json(&url)?;
                for item in page.items {
                    if let Some(meta) = item.into_meta() {
                        metas.push(meta);
                    }
                }
                page_url = page.next;
            }
            Ok((Some(album.name), metas))
        }
        SpotifyKind::Playlist => {
            let playlist_url = format!("{API_BASE}/playlists/{}?fields=name", spotify_ref.id);
            let playlist: NamedResource = client.get_json(&playlist_url)?;
            let mut page_url = Some(format!(
                "{API_BASE}/playlists/{}/tracks?limit=50",
                spotify_ref.id
            ));
            let mut metas = Vec::new();
            while let Some(url) = page_url {
                let page: PlaylistTracksPage = client.get_json(&url)?;
                for item in page.items {
                    if let Some(track) = item.track {
                        if let Some(meta) = track.into_meta() {
                            metas.push(meta);
                        }
                    }
                }
                page_url = page.next;
            }
            Ok((Some(playlist.name), metas))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_playlist() {
        let r =
            parse_spotify_ref("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc")
                .unwrap();
        assert_eq!(r.kind, SpotifyKind::Playlist);
        assert_eq!(r.id, "37i9dQZF1DXcBWIGoYBM5M");
    }

    #[test]
    fn parse_album_and_track_and_uri() {
        let album = parse_spotify_ref("https://open.spotify.com/album/5abc").unwrap();
        assert_eq!(album.kind, SpotifyKind::Album);
        assert_eq!(album.id, "5abc");

        let track =
            parse_spotify_ref("https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC").unwrap();
        assert_eq!(track.kind, SpotifyKind::Track);

        let uri = parse_spotify_ref("spotify:playlist:37i9dQZF1DX0XUsuxWHRQd").unwrap();
        assert_eq!(uri.kind, SpotifyKind::Playlist);
        assert_eq!(uri.id, "37i9dQZF1DX0XUsuxWHRQd");
    }

    #[test]
    fn reject_youtube_as_spotify() {
        assert!(parse_spotify_ref("https://www.youtube.com/playlist?list=PLx").is_err());
        assert!(!is_spotify_url("https://www.youtube.com/watch?v=abc"));
    }

    #[test]
    fn mirror_query_joins_artists_and_title() {
        assert_eq!(
            mirror_search_query("Song", &["A".into(), "B".into()]),
            "A B Song"
        );
        assert_eq!(mirror_search_query("Solo", &[]), "Solo");
    }
}
