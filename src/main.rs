mod error;
mod event;
mod player;
mod spotify;
mod widgets;
mod youtube;

use crate::{
    error::Error,
    event::{Event, Events},
};
use futures::future::join_all;
use librespot::metadata::Metadata;
use player::Player;
use rspotify_model::Id;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use spotify::{SpotifyClient, SpotifyPlayer};
use std::{borrow::BorrowMut, collections::HashSet, fmt::Display, io, iter::FromIterator};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use unicode_width::UnicodeWidthStr;
use widgets::StatefulList;
use youtube::YoutubeClient;

pub type NeoResult<T> = Result<T, Error>;

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// Search results
    results: (StatefulList<Track>, Platform),
    /// Currently playing song
    np: String,
    /// Spotify client
    client: SpotifyClient,
    ///Spotify Player
    /// Queue
    queue: Vec<Track>,
    history: Vec<Track>,
    current: Option<Track>,
    toggle_queue: bool,
    paused: bool,
    player: Player,
    os_media_controls: Option<MediaControls>,
}

#[derive(PartialEq)]
pub enum Platform {
    Spotify,
    Youtube,
}

#[derive(Clone, Debug)]
pub enum Uri {
    Spotify(String),
    Youtube(String),
}

#[derive(Clone, Debug)]
pub struct Track {
    /// Track title
    name: String,
    /// Track artist
    artist: String,
    /// Track URI
    uri: Uri,
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.artist)
    }
}

impl Track {
    fn new(name: String, artist: String, uri: Uri) -> Self {
        Self { name, artist, uri }
    }
}

enum Command {
    /// Unknown Command
    Unknown,
    /// Search spotify for the provided query
    Search(String),
    /// Play the first track returned by spotify for the provided query
    Play(String),
    /// Get the songs saved in the user's library
    Library,
    Pause,
    YTSearch(String),
}

impl From<String> for Command {
    fn from(command: String) -> Self {
        let (prefix, command) = if let Some(split) = command.as_str().split_once(' ') {
            split
        } else {
            (command.as_str(), "")
        };
        match prefix {
            "search" => Self::Search(String::from(command)),
            "play" => Self::Play(String::from(command)),
            "library" => Self::Library,
            "pause" => Self::Pause,
            "ytsearch" => Self::YTSearch(String::from(command)),
            _ => Self::Unknown,
        }
    }
}

impl App {
    async fn handle_command(&mut self) -> Result<(), Error> {
        match Command::from(self.input.drain(..).collect::<String>()) {
            Command::Unknown => {}
            Command::Search(query) => {
                self.results.0.items = self
                    .client
                    .search(query)
                    .await?
                    .into_iter()
                    .map(|track| -> Option<Track> {
                        Some(Track::new(
                            track.name,
                            track.artists.first()?.name.clone(),
                            Uri::Spotify(track.id.unwrap().uri()),
                        ))
                    })
                    .collect::<Option<Vec<Track>>>()
                    .ok_or_else(|| Error::Other(String::from("Search returned no results.")))?;
                self.results.1 = Platform::Spotify;
            }
            Command::YTSearch(query) => {
                self.results.0.items = YoutubeClient::search(query)?
                    .into_iter()
                    .map(|yt_res| {
                        Track::new(yt_res.title, String::new(), Uri::Youtube(yt_res.href))
                    })
                    .collect();
                self.results.1 = Platform::Youtube;
            }
            Command::Play(query) => {
                self.player
                    .play(Uri::Spotify(
                        self.client
                            .search(query)
                            .await?
                            .clone()
                            .first()
                            .expect("No results")
                            .id
                            .clone()
                            .unwrap()
                            .to_string()
                            .clone(),
                    ))
                    .await;
                self.set_playback_state(false)?;
                // TODO: Add to self.current
            }

            Command::Pause => {
                if self.paused {
                    self.player.resume();
                    self.set_playback_state(false)?;
                } else {
                    self.player.pause();
                    self.set_playback_state(true)?;
                }
            }

            Command::Library => {
                self.results.0.items = self
                    .client
                    .clone()
                    .get_library()
                    .await
                    .into_iter()
                    .filter_map(|track| {
                        if let (Some(first_artist), Some(id)) = (track.artists.first(), track.id) {
                            Some(Track::new(
                                track.name,
                                first_artist.name.clone(),
                                Uri::Spotify(id.uri()),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                self.results.1 = Platform::Spotify;
            }
        }
        Ok(())
    }

    fn set_playback_state(&mut self, paused: bool) -> NeoResult<()> {
        self.paused = paused;
        if let Some(controls) = self.os_media_controls.borrow_mut() {
            controls.set_playback(if paused {
                MediaPlayback::Paused { progress: None }
            } else {
                MediaPlayback::Playing { progress: None }
            })?
        };
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let player = SpotifyPlayer::new().await?;
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();

    let mut app = App {
        client: SpotifyClient::new({
            let token = player.get_token();
            rspotify::Token {
                access_token: token.access_token.clone(),
                scopes: HashSet::from_iter(token.scopes.clone().into_iter()),
                ..Default::default()
            }
        })
        .await,
        player: Player::new(handle).await?,
        input: String::new(),
        input_mode: InputMode::Normal,
        results: (StatefulList::new(), Platform::Spotify),
        queue: vec![],
        history: vec![],
        current: None,
        np: String::new(),
        toggle_queue: true,
        paused: true,
        // TODO: Find a way to make this async, or initialize the app without it at first (it takes considerable time to load and delays the app start)
        os_media_controls: None,
    };

    // Terminal initialization
    let stdout = AlternateScreen::from(MouseTerminal::from(io::stdout().into_raw_mode()?));
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let events = Events::new(app.player.spotify.get_event_channel());

    if let Ok(controls) = MediaControls::new(PlatformConfig {
        dbus_name: "neoplayer",
        display_name: "Neoplayer Ultimate",
        hwnd: {
            #[cfg(not(target_os = "windows"))]
            {
                None
            }

            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::windows::WindowsHandle;

                let handle: WindowsHandle = unimplemented!();
                Some(handle.hwnd)
            };
        },
    }) {
        app.os_media_controls = Some(controls)
    };

    let tx_clone = events.tx.clone();

    if let Some(controls) = app.os_media_controls.borrow_mut() {
        controls.attach(move |event: MediaControlEvent| match event {
            MediaControlEvent::Play => tx_clone.send(Event::PleaseResume).unwrap(),
            MediaControlEvent::Pause => tx_clone.send(Event::PleasePause).unwrap(),
            MediaControlEvent::Toggle => todo!(),
            MediaControlEvent::Next => todo!(),
            MediaControlEvent::Previous => todo!(),
            MediaControlEvent::Stop => todo!(),
            MediaControlEvent::Seek(_) => todo!(),
            MediaControlEvent::SeekBy(_, _) => todo!(),
            MediaControlEvent::SetPosition(_) => todo!(),
            MediaControlEvent::OpenUri(_) => todo!(),
            MediaControlEvent::Raise => todo!(),
            MediaControlEvent::Quit => todo!(),
        })?
    };

    loop {
        // Draw UI
        #[cfg(not(feature = "debug-log"))]
        terminal.draw(|f| {
            let master_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(2)
                .constraints(if app.toggle_queue && f.size().width > 90 {
                    [Constraint::Percentage(65), Constraint::Max(30)].as_ref()
                } else {
                    [Constraint::Percentage(100)].as_ref()
                })
                .split(f.size());

            let chunks_left = Layout::default()
                .direction(Direction::Vertical)
                //.margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(master_chunks[0]);

            let np = Paragraph::new(app.np.as_ref()).block(
                Block::default().borders(Borders::ALL).title(format!(
                    "{} {}",
                    if app.player.current == Platform::Spotify {
                        ""
                    } else {
                        ""
                    },
                    if app.paused { "Paused" } else { "Now Playing" }
                )),
            );
            f.render_widget(np, chunks_left[0]);

            let input = Paragraph::new(app.input.as_ref())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::LightGreen),
                })
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input, chunks_left[2]);
            match app.input_mode {
                InputMode::Normal => {}

                InputMode::Editing => f.set_cursor(
                    chunks_left[2].x + app.input.width() as u16 + 1,
                    chunks_left[2].y + 1,
                ),
            }

            let list: Vec<ListItem> = app
                .results
                .0
                .items
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
                    ListItem::new(content)
                })
                .collect();
            let results = List::new(list)
                .block(Block::default().borders(Borders::ALL).title("Tracks"))
                .highlight_style(
                    Style::default()
                        .bg(if app.results.1 == Platform::Spotify {
                            Color::LightGreen
                        } else {
                            Color::LightRed
                        })
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(results, chunks_left[1], &mut app.results.0.state);

            let tracks: Vec<ListItem> = app
                .queue
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
                    ListItem::new(content)
                })
                .collect();
            let queue = List::new(tracks)
                .block(Block::default().borders(Borders::ALL).title("Queue"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                );

            if app.toggle_queue && f.size().width > 90 {
                f.render_widget(queue, master_chunks[1]);
            }
        })?;

        if let (Some(controls), Some(current)) =
            (app.os_media_controls.borrow_mut(), app.current.clone())
        {
            controls.set_metadata(MediaMetadata {
                title: Some(&current.name),
                artist: Some(&current.artist),
                ..Default::default()
            })?;
        }

        if app.player.current == Platform::Youtube && app.player.youtube.sink.empty() {
            if let Some(next) = app.queue.first() {
                app.player.play(next.uri.clone()).await;
                if let Uri::Youtube(_) = next.uri {
                    app.np = next.name.clone()
                }
                app.queue.remove(0);
                app.set_playback_state(false)?;
            }
        }

        // Handle input
        match events.next()? {
            Event::Input(input) => match app.input_mode {
                InputMode::Normal => match input {
                    Key::Char('h') => {
                        app.input_mode = InputMode::Editing;
                    }
                    Key::Down => {
                        app.results.0.next();
                    }
                    Key::Up => {
                        app.results.0.previous();
                    }
                    Key::Char('e') => {
                        if let Some(controls) = app.os_media_controls.borrow_mut() {
                            controls.detach()?;
                        }
                        return Ok(());
                    }
                    Key::Char('\n') => {
                        app.player
                            .play(app.results.0.get_selection().uri.clone())
                            .await;
                        if let Uri::Youtube(_) = app.results.0.get_selection().uri.clone() {
                            app.np = app.results.0.get_selection().name.clone();
                        }
                        app.set_playback_state(false)?;
                        app.current = Some(app.results.0.get_selection().clone());
                    }
                    Key::Char('a') => {
                        let selection = &(*app.results.0.get_selection());
                        app.queue.push(Track::new(
                            selection.name.to_string(),
                            selection.artist.to_string(),
                            selection.uri.clone(),
                        ));
                    }
                    Key::Char('q') => {
                        app.toggle_queue = !app.toggle_queue;
                    }
                    Key::Char(',') => {
                        if let Some(previous) = app.history.first() {
                            app.player.play(previous.uri.clone()).await;
                            if let Uri::Youtube(_) = previous.uri {
                                app.np = previous.name.clone()
                            }
                            if let Some(current) = app.current.clone() {
                                app.queue.insert(0, current);
                            }
                            app.current = Some(previous.clone());
                            app.history.remove(0);
                            app.set_playback_state(false)?;
                        }
                    }
                    Key::Char('.') => {
                        if let Some(next) = app.queue.first() {
                            app.player.play(next.uri.clone()).await;
                            if let Uri::Youtube(_) = next.uri {
                                app.np = next.name.clone()
                            }
                            if let Some(current) = app.current.clone() {
                                app.history.insert(0, current);
                            }
                            app.current = Some(next.clone());
                            app.queue.remove(0);
                            app.set_playback_state(false)?;
                        }
                    }
                    _ => {}
                },
                InputMode::Editing => match input {
                    Key::Char('\n') => {
                        app.handle_command().await?;
                        app.input_mode = InputMode::Normal;
                    }
                    Key::Char(c) => {
                        app.input.push(c);
                    }
                    Key::Backspace => {
                        app.input.pop();
                    }
                    Key::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
            },

            Event::UpdateNP(track) => {
                app.np = {
                    let data =
                        librespot::metadata::Track::get(app.player.spotify.get_session(), track)
                            .await?;
                    format!(
                        "{} - {}",
                        join_all(data.artists.iter().map(|id| {
                            let cloned_session = app.player.spotify.get_session().clone();

                            async move {
                                librespot::metadata::Artist::get(&cloned_session, *id)
                                    .await
                                    .unwrap()
                                    .name
                            }
                        }))
                        .await
                        .join(", "),
                        data.name
                    )
                }
            }

            Event::TrackEnded => {
                if let Some(next) = app.queue.first() {
                    app.player.play(next.uri.clone()).await;
                    if let Uri::Youtube(_) = next.uri {
                        app.np = next.name.clone()
                    }
                    app.queue.remove(0);
                    app.set_playback_state(false)?;
                } else if let Some(controls) = app.os_media_controls.borrow_mut() {
                    controls.set_playback(MediaPlayback::Stopped)?
                }
            }

            Event::PleasePause => {
                app.player.pause();
                app.set_playback_state(true)?;
            }
            Event::PleaseResume => {
                app.player.resume();
                app.set_playback_state(false)?;
            }

            Event::Tick => (),
        }
    }
}
