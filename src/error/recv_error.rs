// This is free and unencumbered software released into the public domain.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("RecvError")]
pub struct RecvError;

#[cfg(feature = "flume")]
impl From<flume::RecvError> for RecvError {
    fn from(_input: flume::RecvError) -> Self {
        Self // TODO
    }
}
