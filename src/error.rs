use std::io;

use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

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
    #[error("Error sending input to connection thread")]
    Send(#[from] SendError<u32>)
}

pub type Result<T> = std::result::Result<T, Error>;
