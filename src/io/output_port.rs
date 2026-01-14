// This is free and unencumbered software released into the public domain.

use crate::io::SendError;
use alloc::boxed::Box;

#[async_trait::async_trait]
pub trait OutputPort<T: Send + 'static> {
    async fn send(&self, value: T) -> Result<(), SendError>;

    // TODO: try_send
}
