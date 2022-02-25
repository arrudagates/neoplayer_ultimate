use std::{
    fmt::{Debug, Display, Formatter},
    sync::mpsc::RecvError,
};

use rspotify::ClientError;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Dotenv(dotenv::Error),
    LibreSpot(librespot::core::Error),
    Receiver(RecvError),
    Client(ClientError),
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
