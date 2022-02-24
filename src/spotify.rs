use librespot::{
    core::{
        authentication::Credentials, config::SessionConfig, session::Session,
        spotify_id::SpotifyId, token::Token,
    },
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        player::{Player, PlayerEventChannel},
    },
};
use rspotify::{prelude::*, AuthCodeSpotify};
use rspotify_model::{
    enums::types::SearchType, page::Page, search::SearchResult, track::FullTrack,
};

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: AuthCodeSpotify,
}

pub struct SpotifyPlayer {
    player: Player,
    session: Session,
    token: Token,
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

        let session = Session::new(session_config, None);
        session.connect(credentials).await.unwrap();

        let (player, _) = Player::new(player_config, session.clone(), None, move || {
            backend(None, audio_format)
        });

        let token = session.token_provider().get_token("app-remote-control,streaming,user-library-read,user-read-currently-playing,user-read-playback-state,user-read-playback-position,playlist-read-collaborative,playlist-read-private,user-library-modify,user-modify-playback-state").await.unwrap();

        Self {
            player,
            session,
            token,
        }
    }

    pub fn get_event_channel(&self) -> PlayerEventChannel {
        self.player.get_player_event_channel()
    }

    pub fn get_session(&self) -> &Session {
        &self.session
    }

    pub fn get_token(&self) -> &Token {
        &self.token
    }

    pub async fn play(&mut self, uri: String) {
        self.player
            .load(SpotifyId::from_uri(&uri).unwrap(), true, 0);
        self.player.play();
    }
}

impl SpotifyClient {
    pub async fn new(token: rspotify::Token) -> Self {
        Self {
            client: rspotify::AuthCodeSpotify::from_token(token),
        }
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
