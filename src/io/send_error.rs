// This is free and unencumbered software released into the public domain.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("SendError")]
pub struct SendError;

#[cfg(feature = "flume")]
impl<T> From<flume::SendError<T>> for SendError {
    fn from(_input: flume::SendError<T>) -> Self {
        Self // TODO
    }
}

#[cfg(feature = "tokio")]
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError {
    fn from(_input: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self // TODO
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("TrySendError")]
pub struct TrySendError;

#[cfg(feature = "flume")]
impl<T> From<flume::TrySendError<T>> for TrySendError {
    fn from(_input: flume::TrySendError<T>) -> Self {
        Self // TODO
    }
}

#[cfg(feature = "tokio")]
impl<T> From<tokio::sync::mpsc::error::TrySendError<T>> for TrySendError {
    fn from(_input: tokio::sync::mpsc::error::TrySendError<T>) -> Self {
        Self // TODO
    }
}
