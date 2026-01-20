// This is free and unencumbered software released into the public domain.

use crate::error::SendError;
use alloc::boxed::Box;

#[async_trait::async_trait]
pub trait OutputPort<T: Send + 'static> {
    async fn send(&self, message: T) -> Result<(), SendError>;

    // TODO: send_event
    // TODO: send_deadline
    // TODO: send_timeout
    // TODO: try_send
}
