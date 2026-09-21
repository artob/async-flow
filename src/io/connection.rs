// This is free and unencumbered software released into the public domain.

use core::any::TypeId;

/// A message-carrying connection between an output port and an input port.
///
/// Backend-specific channel types provide the transport. This trait exposes
/// the Rust type of the message payload.
#[async_trait::async_trait]
pub trait Connection<T: 'static> {
    /// Returns the Rust type ID of the message payload `T`.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}
