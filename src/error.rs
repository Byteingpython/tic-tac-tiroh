use std::io;

use thiserror::Error;
use tokio::task::JoinError;

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
    InputAbort
}

pub type Result<T> = std::result::Result<T, Error>;
