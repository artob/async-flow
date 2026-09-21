// This is free and unencumbered software released into the public domain.

use crate::{error::RecvError, io::Port};
use alloc::{boxed::Box, vec::Vec};
use core::any::TypeId;

/// A receiving interface for message payloads of type `T`.
#[async_trait::async_trait]
pub trait InputPort<T: Send + 'static>: Port<T> {
    /// Returns the Rust type ID of the message payload.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    /// Checks if this port is empty.
    fn is_empty(&self) -> bool;

    /// Receives a message, or `Ok(None)` at the end of the connection's stream.
    ///
    /// See the backend's lifecycle contract for control-event handling, errors,
    /// and cancellation behavior.
    async fn recv(&mut self) -> Result<Option<T>, RecvError>;

    /// Collects messages until EOF, returning any receive error encountered.
    ///
    /// This can wait indefinitely for an open connection and buffers all received
    /// messages in memory. It is not cancellation-safe: cancellation or an error
    /// drops messages already accumulated by this call.
    async fn recv_all(&mut self) -> Result<Vec<T>, RecvError> {
        let mut inputs = Vec::new();
        while let Some(input) = self.recv().await? {
            inputs.push(input);
        }
        Ok(inputs)
    }

    // TODO: recv_event
    // TODO: recv_deadline
    // TODO: recv_timeout
    // TODO: try_recv
    // TODO: into_stream
}
