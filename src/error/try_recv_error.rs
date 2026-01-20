// This is free and unencumbered software released into the public domain.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("TryRecvError")]
pub struct TryRecvError;

#[cfg(feature = "flume")]
impl From<flume::TryRecvError> for TryRecvError {
    fn from(_input: flume::TryRecvError) -> Self {
        Self // TODO
    }
}

#[cfg(feature = "tokio")]
impl From<tokio::sync::mpsc::error::TryRecvError> for TryRecvError {
    fn from(_input: tokio::sync::mpsc::error::TryRecvError) -> Self {
        Self // TODO
    }
}
