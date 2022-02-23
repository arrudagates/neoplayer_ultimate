use librespot::{
    core::{
        authentication::Credentials, config::SessionConfig, session::Session, spotify_id::SpotifyId,
    },
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        player::{Player, PlayerEventChannel},
    },
};
use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config};
use rspotify_model::{
    enums::types::SearchType, page::Page, search::SearchResult, track::FullTrack,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: AuthCodeSpotify,
}

pub struct SpotifyPlayer {
    player: Player,
    session: Session,
}

impl SpotifyPlayer {
    pub async fn new() -> Self {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        // TODO: Replace normal credentials with OAuth
        let credentials = Credentials::with_password(
            dotenv::var("USERNAME").unwrap(),
            dotenv::var("PASSWORD").unwrap(),
        );

        let backend = audio_backend::find(None).unwrap();

        let session = Session::connect(session_config, credentials, None)
            .await
            .unwrap();

        let (player, _) = Player::new(player_config, session.clone(), None, move || {
            backend(None, audio_format)
        });

        Self { player, session }
    }

    pub fn get_event_channel(&self) -> PlayerEventChannel {
        self.player.get_player_event_channel()
    }

    pub fn get_session(&self) -> &Session {
        &self.session
    }
}

impl SpotifyClient {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();

        let scope = scopes!(
            "app-remote-control",
            "streaming",
            "user-library-read",
            "user-read-currently-playing",
            "user-read-playback-state",
            "user-read-playback-position",
            "playlist-read-collaborative",
            "playlist-read-private",
            "user-library-modify",
            "user-modify-playback-state"
        );

        let creds = rspotify::Credentials::from_env().unwrap();

        let oauth = rspotify::OAuth::from_env(scope).unwrap();

        let mut spotify = AuthCodeSpotify::with_config(
            creds,
            oauth,
            Config {
                cache_path: Path::new("./spotify_cache").to_path_buf(),
                token_cached: true,
                token_refreshing: true,
                ..Default::default()
            },
        );

        let url = spotify.get_authorize_url(false).unwrap();

        if spotify.refresh_token().await.is_err() {
            spotify.prompt_for_token(&url).await.unwrap()
        }

        Self { client: spotify }
    }

    pub async fn play(&mut self, player: &mut SpotifyPlayer, uri: String) {
        player
            .player
            .load(SpotifyId::from_uri(&uri).unwrap(), true, 0);
        player.player.play();
    }

    pub async fn search(&self, query: String) -> Vec<FullTrack> {
        if let SearchResult::Tracks(page) = &self
            .client
            .search(&query, &SearchType::Track, None, None, Some(20), None)
            .await
            .unwrap()
        {
            page.items.clone()
        } else {
            vec![]
        }
    }

    // TODO: Implement paging instead of fetching all tracks at once
    pub async fn get_library(&mut self) -> Vec<FullTrack> {
        let mut library = vec![];
        let mut offset = 0;
        while let Ok(Page { items, total, .. }) = self
            .client
            .current_user_saved_tracks_manual(None, Some(50), Some(offset))
            .await
        {
            library.extend(items.into_iter().map(|saved| saved.track));
            if offset > total {
                break;
            }
            offset += 50;
        }
        library
    }
}
