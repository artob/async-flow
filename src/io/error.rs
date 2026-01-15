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

    #[cfg(feature = "std")]
    #[error("{0}")]
    Stdio(#[from] std::io::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
