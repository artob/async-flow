// This is free and unencumbered software released into the public domain.

use thiserror::Error;

/// A receive failure, including premature termination of a required stream.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RecvError {
    /// A backend could not receive a message.
    #[error("RecvError")]
    Unavailable,
    /// The stream ended before its minimum number of messages was received.
    #[error("expected at least {minimum} messages, received {received}")]
    CardinalityUnderflow {
        /// The required minimum for the whole stream.
        minimum: usize,
        /// Messages already received through this endpoint.
        received: usize,
    },
}

impl From<crate::io::PortState> for RecvError {
    fn from(_input: crate::io::PortState) -> Self {
        Self::Unavailable
    }
}

#[cfg(feature = "flume")]
impl From<flume::RecvError> for RecvError {
    fn from(_input: flume::RecvError) -> Self {
        Self::Unavailable // TODO
    }
}
