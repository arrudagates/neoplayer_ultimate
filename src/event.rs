use librespot::{core::spotify_id::SpotifyId, playback::player::PlayerEventChannel};
use std::{io, sync::mpsc, thread, time::Duration};
use termion::{event::Key, input::TermRead};

#[derive(Debug)]
pub enum Event<I> {
    Input(I),
    Tick,
    UpdateNP(SpotifyId),
    TrackEnded,
    PleasePause,
    PleaseResume,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    pub tx: mpsc::Sender<Event<Key>>,
    rx: mpsc::Receiver<Event<Key>>,
}

impl Events {
    pub fn new(mut player_events: PlayerEventChannel) -> Events {
        let (tx, rx) = mpsc::channel();
        {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for key in stdin.keys().flatten() {
                    if let Err(err) = tx.send(Event::Input(key)) {
                        eprintln!("{}", err);
                        return;
                    }
                }
            })
        };
        let tx_clone = tx.clone();

        thread::spawn(move || loop {
            if let Err(err) = tx_clone.send(Event::Tick) {
                eprintln!("{}", err);
                break;
            }
            thread::sleep(Duration::from_millis(250));
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(event) = player_events.recv().await {
                match event {
                    librespot::playback::player::PlayerEvent::Stopped { .. } => (),
                    librespot::playback::player::PlayerEvent::Started { track_id, .. } => {
                        tx_clone.send(Event::UpdateNP(track_id)).unwrap()
                    }
                    librespot::playback::player::PlayerEvent::Changed { new_track_id, .. } => {
                        tx_clone.send(Event::UpdateNP(new_track_id)).unwrap()
                    }
                    librespot::playback::player::PlayerEvent::Loading { .. } => (),
                    librespot::playback::player::PlayerEvent::Preloading { .. } => (),
                    librespot::playback::player::PlayerEvent::Playing { .. } => (),
                    librespot::playback::player::PlayerEvent::Paused { .. } => (),
                    librespot::playback::player::PlayerEvent::TimeToPreloadNextTrack { .. } => (),
                    librespot::playback::player::PlayerEvent::EndOfTrack { .. } => {
                        tx_clone.send(Event::TrackEnded).unwrap()
                    }
                    librespot::playback::player::PlayerEvent::Unavailable { .. } => (),
                    librespot::playback::player::PlayerEvent::VolumeSet { .. } => (),
                }
            }
        });

        Events { tx, rx }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
