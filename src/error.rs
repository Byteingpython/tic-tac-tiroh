use std::io;

use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

use crate::rock_paper_scissor::Guess;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Connection lost: {0}")]
    Connection(#[from] iroh::endpoint::ConnectionError),
    #[error("Error reading from connection: {0}")]
    Read(#[from] iroh::endpoint::ReadError),
    #[error("Error writing to connection: {0}")]
    Write(#[from] iroh::endpoint::WriteError),
    #[error("Error joining on input thread: {0}")]
    JoinError(#[from] JoinError),
    #[error("Input thread stopped prematurely")]
    InputAbort,
    #[error("Error sending input to connection thread: {0}")]
    SendNumber(#[from] SendError<u32>),
    #[error("Error sending input to connection thread: {0}")]
    SendGuess(#[from] SendError<Guess>),
    #[error("There was an error converting to internal types")]
    ConversionError,
    #[error("Error encrypting guess: {0}")]
    CryptoError(chacha20poly1305::Error),
    #[error("Error: Buffer size incorrect")]
    SizeError,
}

impl From<chacha20poly1305::Error> for Error {
    fn from(value: chacha20poly1305::Error) -> Self {
        return Error::CryptoError(value);
    }
}

pub type Result<T> = std::result::Result<T, Error>;
