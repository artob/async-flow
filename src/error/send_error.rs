// This is free and unencumbered software released into the public domain.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("SendError")]
pub enum SendError {
    #[error("failed to send message on unconnected port")]
    Unconnected,

    #[error("failed to send message on disconnected port")]
    Disconnected,

    #[error("failed to send message on closed port")]
    Closed,
}

impl From<crate::io::PortState> for SendError {
    fn from(input: crate::io::PortState) -> Self {
        use crate::io::PortState::*;
        match input {
            Unconnected => Self::Unconnected,
            Connected => unreachable!(),
            Disconnected => Self::Disconnected,
            Closed => Self::Closed,
        }
    }
}

#[cfg(feature = "flume")]
impl<T> From<flume::SendError<T>> for SendError {
    fn from(_input: flume::SendError<T>) -> Self {
        Self // TODO
    }
}

#[cfg(feature = "tokio")]
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError {
    fn from(_input: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Disconnected
    }
}
