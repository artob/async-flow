// This is free and unencumbered software released into the public domain.

use super::{RecvError, SendError, TryRecvError, TrySendError};
use alloc::boxed::Box;
use thiserror::Error;

pub type Result<T = (), E = Error> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Recv(#[from] RecvError),

    #[error("{0}")]
    TryRecv(#[from] TryRecvError),

    #[error("{0}")]
    Send(#[from] SendError),

    #[error("{0}")]
    TrySend(#[from] TrySendError),

    #[cfg(feature = "tokio")]
    #[error("{0}")]
    Join(#[from] tokio::task::JoinError),

    /// System preparation failed before any block processes were started.
    #[cfg(feature = "tokio")]
    #[error("{0}")]
    Prepare(#[from] crate::tokio::SystemPrepareError),

    /// A typed runtime boundary could not be bound.
    #[cfg(feature = "tokio")]
    #[error("{0}")]
    Bind(#[from] crate::tokio::PortBindingError),

    /// Prepared processes were executed without an active Tokio runtime.
    #[cfg(feature = "tokio")]
    #[error("{0}")]
    Runtime(#[from] tokio::runtime::TryCurrentError),

    #[cfg(feature = "std")]
    #[error("{0}")]
    Stdio(#[from] std::io::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn core::error::Error + Send + Sync>),
}

#[cfg(feature = "serde")]
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Other(Box::new(value))
    }
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Error::Other(Box::new(value))
    }
}
