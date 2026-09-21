// This is free and unencumbered software released into the public domain.

use crate::{Port, error::SendError};
use alloc::boxed::Box;
use core::any::TypeId;

/// A sending interface for message payloads of type `T`.
///
/// The [`Port`] supertrait exposes the same lifecycle, capacity, and cardinality
/// queries available on input trait objects.
#[async_trait::async_trait]
pub trait OutputPort<T: Send + 'static>: Port<T> {
    /// Returns the Rust type ID of the message payload.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    /// Sends a message according to the backend's delivery and backpressure rules.
    ///
    /// See the backend for error, payload ownership, and cancellation guarantees.
    async fn send(&self, message: T) -> Result<(), SendError>;

    // TODO: send_event
    // TODO: send_deadline
    // TODO: send_timeout
    // TODO: try_send
}
