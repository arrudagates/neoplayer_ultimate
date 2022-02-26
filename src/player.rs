use rodio::OutputStreamHandle;

use crate::{spotify::SpotifyPlayer, youtube::YoutubeClient, Platform, Uri};

pub struct Player {
    pub youtube: YoutubeClient,
    pub spotify: SpotifyPlayer,
    pub current: Platform,
}

impl Player {
    pub async fn new(osh: OutputStreamHandle) -> Self {
        Self {
            youtube: YoutubeClient::new(osh),
            spotify: SpotifyPlayer::new().await.unwrap(),
            current: Platform::Spotify,
        }
    }

    pub async fn play(&mut self, uri: Uri) {
        match uri {
            Uri::Spotify(uri) => {
                self.spotify.play(uri).await.unwrap();
                self.current = Platform::Spotify
            }
            Uri::Youtube(video_id) => {
                self.youtube.play(video_id).await;
                self.current = Platform::Youtube
            }
        };
    }

    pub fn pause(&mut self) {
        match self.current {
            Platform::Spotify => self.spotify.pause(),
            Platform::Youtube => self.youtube.pause(),
        }
    }

    pub fn resume(&mut self) {
        match self.current {
            Platform::Spotify => self.spotify.resume(),
            Platform::Youtube => self.youtube.resume(),
        }
    }
}
