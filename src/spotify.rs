use dotenv;
use rspotify::client::Spotify;

use rspotify::client::SpotifyBuilder;
use rspotify::model::enums::types::SearchType;
use rspotify::model::page::Page;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::Id;
use rspotify::oauth2::{CredentialsBuilder, OAuthBuilder};
use rspotify::scopes;

use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::playback::audio_backend;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::player::Player;
use librespot::protocol::authentication::AuthenticationType;

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: Spotify,
}

pub struct SpotifyPlayer {
    player: Player,
}

impl SpotifyPlayer {
    pub async fn new(spotify: SpotifyClient) -> Self {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        let credentials = Credentials {
            username: spotify.client.me().await.unwrap().id,
            auth_type: AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN,
            auth_data: spotify
                .client
                .token
                .as_ref()
                .unwrap()
                .access_token
                .as_bytes()
                .to_vec(),
        };

        let backend = audio_backend::find(None).unwrap();

        let session = Session::connect(session_config, credentials, None)
            .await
            .unwrap();

        let (player, _) = Player::new(player_config, session, None, move || {
            backend(None, audio_format)
        });
        Self { player }
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

        let creds = CredentialsBuilder::from_env().build().unwrap();

        let oauth = OAuthBuilder::from_env()
            // .client_id(&dotenv::var("RSPOTIFY_CLIENT_ID").unwrap())
            // .client_secret(&dotenv::var("RSPOTIFY_CLIENT_SECRET").unwrap())
            // .redirect_uri(&dotenv::var("RSPOTIFY_REDIRECT_URI").unwrap())
            .scope(scope)
            .build()
            .unwrap();

        //let token = get_token(&mut oauth).await.unwrap();

        //let client = Spotify::default().access_token(&token.access_token);
        let mut spotify = SpotifyBuilder::default()
            .credentials(creds)
            .oauth(oauth)
            .build()
            .unwrap();

        spotify.prompt_for_user_token().await.unwrap();

        Self {
            client: spotify,
            //  player,
        }
    }

    pub async fn play(&mut self, player: &mut SpotifyPlayer, uri: String) {
        player
            .player
            .load(SpotifyId::from_uri(&uri).unwrap(), true, 0);
        //  let id: Id<Track> = Id::from(uri);
        //  self.client
        //      .start_uris_playback(vec![id], None, None, None)
        //      .await
        //      .unwrap();
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
            .current_user_saved_tracks_manual(Some(50), Some(offset))
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
