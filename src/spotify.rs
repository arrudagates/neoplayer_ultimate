use dotenv;
use rspotify::client::Spotify;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::oauth2::SpotifyOAuth;
use rspotify::senum::SearchType;
use rspotify::util::get_token;

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: Spotify,
}

impl SpotifyClient {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();

        let mut oauth = SpotifyOAuth::default()
            .client_id(&dotenv::var("RSPOTIFY_CLIENT_ID").unwrap())
            .client_secret(&dotenv::var("RSPOTIFY_CLIENT_SECRET").unwrap())
            .redirect_uri(&dotenv::var("RSPOTIFY_REDIRECT_URI").unwrap())
            .scope("app-remote-control streaming user-library-read user-read-currently-playing user-read-playback-state user-read-playback-position playlist-read-collaborative playlist-read-private user-library-modify user-modify-playback-state")
            .build();

        let token = get_token(&mut oauth).await.unwrap();

        Self {
            client: Spotify::default().access_token(&token.access_token),
        }
    }

    pub async fn play(&self, uri: String) {
        self.client
            .start_playback(None, None, Some(vec![uri]), None, None)
            .await
            .unwrap();
    }

    pub async fn search(&self, query: String) -> Vec<FullTrack> {
        if let SearchResult::Tracks(page) = &self
            .client
            .search(&query, SearchType::Track, Some(20), None, None, None)
            .await
            .unwrap()
        {
            page.items.clone()
        } else {
            vec![]
        }
    }

    pub async fn get_library(&self) -> Vec<FullTrack> {
        self.client
            .current_user_saved_tracks(None, None)
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|saved| saved.track)
            .collect()
    }
}
