// This is free and unencumbered software released into the public domain.

use alloc::boxed::Box;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("SendError")]
pub struct SendError;

#[cfg(feature = "tokio")]
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError {
    fn from(input: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self // TODO
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("TrySendError")]
pub struct TrySendError;

#[cfg(feature = "tokio")]
impl<T> From<tokio::sync::mpsc::error::TrySendError<T>> for TrySendError {
    fn from(input: tokio::sync::mpsc::error::TrySendError<T>) -> Self {
        Self // TODO
    }
}
