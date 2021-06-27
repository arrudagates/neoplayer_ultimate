mod event;
mod spotify;
mod widgets;

use crate::event::{Event, Events};
use std::{error::Error, io};
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

use rspotify::client::Spotify;
use widgets::StatefulList;

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
    spotify: Option<Spotify>,
}

struct Track {
    name: String,
    uri: String,
}

impl Track {
    fn new(name: String, uri: String) -> Self {
        Self { name, uri }
    }
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            results: StatefulList::new(),
            np: String::new(),
            spotify: None,
        }
    }
}

enum Command {
    Unknown,
    Search(String),
}

impl From<String> for Command {
    fn from(command: String) -> Self {
        let command = command.as_str().split_once(' ').unwrap();
        let prefix = command.0;
        match prefix {
            "search" => Self::Search(String::from(command.1)),
            _ => Self::Unknown,
        }
    }
}

impl App {
    async fn handle_command(&mut self) {
        match Command::from(self.input.drain(..).collect::<String>()) {
            Command::Unknown => {}
            Command::Search(query) => println!(
                "{:?}",
                if let rspotify::model::search::SearchResult::Tracks(page) = &self
                    .spotify
                    .as_ref()
                    .unwrap()
                    .search(
                        &query,
                        rspotify::senum::SearchType::Track,
                        Some(20),
                        None,
                        None,
                        None
                    )
                    .await
                    .unwrap()
                {
                    self.results.items = page
                        .items
                        .clone()
                        .into_iter()
                        .map(|track| Track::new(track.name, track.uri))
                        .collect();
                } else {
                    {}
                }
            ),
        }
    }

    async fn play(&self) {
        self.spotify
            .as_ref()
            .unwrap()
            .start_playback(
                None,
                None,
                Some(vec![self.results.get_selection().uri.clone()]),
                None,
                None,
            )
            .await
            .unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let events = Events::new();

    // Create default app state
    let mut app = App::default();
    app.spotify = Some(spotify::get_spotify_client().await);

    loop {
        // Draw UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let np = Paragraph::new(app.np.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Now Playing"));
            f.render_widget(np, chunks[0]);

            let input = Paragraph::new(app.input.as_ref())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::LightGreen),
                })
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input, chunks[2]);
            match app.input_mode {
                InputMode::Normal =>
                    // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                    {}

                InputMode::Editing => {
                    // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                    f.set_cursor(
                        // Put cursor past the end of the input text
                        chunks[2].x + app.input.width() as u16 + 1,
                        // Move one line down, from the border to the input line
                        chunks[2].y + 1,
                    )
                }
            }

            let list: Vec<ListItem> = app
                .results
                .items
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m.name)))];
                    ListItem::new(content)
                })
                .collect();
            let results = List::new(list)
                .block(Block::default().borders(Borders::ALL).title("Tracks"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                //.highlight_symbol(">> ")
                ;

            f.render_stateful_widget(results, chunks[1], &mut app.results.state);
        })?;

        // Handle input
        if let Event::Input(input) = events.next()? {
            match app.input_mode {
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
                    Key::Char('q') => {
                        break;
                    }
                    Key::Char('\n') => {
                        app.play().await;
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
            }
        }
    }
    Ok(())
}
