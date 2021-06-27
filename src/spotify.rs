use dotenv;
use rspotify::client::Spotify;
use rspotify::oauth2::SpotifyOAuth;
use rspotify::util::get_token;

pub async fn get_spotify_client() -> Spotify {
    dotenv::dotenv().ok();

    let mut oauth = SpotifyOAuth::default()
        .client_id(&dotenv::var("RSPOTIFY_CLIENT_ID").unwrap())
        .client_secret(&dotenv::var("RSPOTIFY_CLIENT_SECRET").unwrap())
        .redirect_uri(&dotenv::var("RSPOTIFY_REDIRECT_URI").unwrap())
        .scope("app-remote-control streaming user-library-read user-read-currently-playing user-read-playback-state user-read-playback-position playlist-read-collaborative playlist-read-private user-library-modify user-modify-playback-state")
        .build();

    let token = get_token(&mut oauth).await.unwrap();

    Spotify::default().access_token(&token.access_token)
}
