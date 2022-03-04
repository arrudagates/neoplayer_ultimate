use std::{
    fmt::{Debug, Display, Formatter},
    sync::mpsc::RecvError,
};

use rspotify::ClientError;
use termion::event::Key;
use tokio::sync::broadcast::error::SendError;

use crate::event::Event;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Dotenv(dotenv::Error),
    LibreSpot(librespot::core::Error),
    EventSender(SendError<Event<Key>>),
    Receiver(RecvError),
    Client(ClientError),
    OSMediaControls(souvlaki::Error),
    Ureq(ureq::Error),
    SerdeJson(serde_json::Error),
    Other(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::IO(source)
    }
}

impl From<dotenv::Error> for Error {
    fn from(source: dotenv::Error) -> Self {
        Error::Dotenv(source)
    }
}

impl From<librespot::core::Error> for Error {
    fn from(source: librespot::core::Error) -> Self {
        Error::LibreSpot(source)
    }
}

impl From<RecvError> for Error {
    fn from(source: RecvError) -> Self {
        Error::Receiver(source)
    }
}

impl From<ClientError> for Error {
    fn from(source: ClientError) -> Self {
        Error::Client(source)
    }
}

impl From<souvlaki::Error> for Error {
    fn from(source: souvlaki::Error) -> Self {
        Error::OSMediaControls(source)
    }
}

impl From<ureq::Error> for Error {
    fn from(source: ureq::Error) -> Self {
        Error::Ureq(source)
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::SerdeJson(source)
    }
}

impl From<SendError<Event<Key>>> for Error {
    fn from(source: SendError<Event<Key>>) -> Self {
        Error::EventSender(source)
    }
}
