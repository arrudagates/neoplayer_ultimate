mod event;
mod spotify;
mod widgets;

use crate::event::{Event, Events};
use librespot::metadata::Metadata;
use rspotify_model::Id;
use spotify::{SpotifyClient, SpotifyPlayer};
use std::{collections::HashSet, error::Error, fmt::Display, io, iter::FromIterator};
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

use futures::future::join_all;

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
    results: StatefulList<Track>,
    /// Currently playing song
    np: String,
    /// Spotify client
    client: SpotifyClient,
    ///Spotify Player
    player: SpotifyPlayer,
    /// Queue
    queue: Vec<Track>,
    toggle_queue: bool,
}

struct Track {
    /// Track title
    name: String,
    /// Track artist
    artist: String,
    /// Track URI
    uri: String,
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.artist)
    }
}

impl Track {
    fn new(name: String, artist: String, uri: String) -> Self {
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
            _ => Self::Unknown,
        }
    }
}

impl App {
    async fn handle_command(&mut self) {
        match Command::from(self.input.drain(..).collect::<String>()) {
            Command::Unknown => {}
            Command::Search(query) => {
                self.results.items = self
                    .client
                    .search(query)
                    .await
                    .into_iter()
                    .map(|track| {
                        Track::new(
                            track.name,
                            track.artists.first().unwrap().name.clone(),
                            track.id.unwrap().uri(),
                        )
                    })
                    .collect()
            }
            Command::Play(query) => {
                self.player
                    .play(
                        self.client
                            .search(query)
                            .await
                            .clone()
                            .first()
                            .expect("No results")
                            .id
                            .clone()
                            .unwrap()
                            .to_string()
                            .clone(),
                    )
                    .await;
            }
            Command::Library => {
                self.results.items = self
                    .client
                    .clone()
                    .get_library()
                    .await
                    .into_iter()
                    .map(|track| {
                        Track::new(
                            track.name,
                            track.artists.first().unwrap().name.clone(),
                            track.id.unwrap().uri(),
                        )
                    })
                    .collect();
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let player = SpotifyPlayer::new().await;

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
        player,
        input: String::new(),
        input_mode: InputMode::Normal,
        results: StatefulList::new(),
        queue: vec![],
        np: String::new(),
        toggle_queue: true,
    };
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let events = Events::new(app.player.get_event_channel());

    loop {
        // Draw UI
        terminal.draw(|f| {
            let master_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(2)
                .constraints(if app.toggle_queue {
                    [Constraint::Min(40), Constraint::Max(30)].as_ref()
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

            let np = Paragraph::new(app.np.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Now Playing"));
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
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(results, chunks_left[1], &mut app.results.state);

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

            if app.toggle_queue {
                f.render_widget(queue, master_chunks[1]);
            }
        })?;

        // Handle input
        match events.next()? {
            Event::Input(input) => match app.input_mode {
                InputMode::Normal => match input {
                    Key::Char('h') => {
                        app.input_mode = InputMode::Editing;
                    }
                    Key::Down => {
                        app.results.next();
                    }
                    Key::Up => {
                        app.results.previous();
                    }
                    Key::Char('e') => {
                        break;
                    }
                    Key::Char('\n') => {
                        app.player
                            .play(app.results.get_selection().uri.clone())
                            .await;
                    }
                    Key::Char('a') => {
                        let selection = &(*app.results.get_selection());
                        app.queue.push(Track::new(
                            selection.name.to_string(),
                            selection.artist.to_string(),
                            selection.uri.to_string(),
                        ));
                    }
                    Key::Char('q') => {
                        app.toggle_queue = !app.toggle_queue;
                    }
                    _ => {}
                },
                InputMode::Editing => match input {
                    Key::Char('\n') => {
                        app.handle_command().await;
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
                    let data = librespot::metadata::Track::get(app.player.get_session(), track)
                        .await
                        .unwrap();
                    format!(
                        "{} - {}",
                        join_all(data.artists.iter().map(|id| {
                            let cloned_session = app.player.get_session().clone();

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
                    app.queue.remove(0);
                }
            }

            Event::Tick => (),
        }
    }
    Ok(())
}
