// This is free and unencumbered software released into the public domain.

use alloc::boxed::Box;
use tokio::sync::mpsc::error::SendError;

#[async_trait::async_trait]
pub trait OutputPort<T: Send + 'static> {
    async fn send(&self, value: T) -> Result<(), SendError<T>>;
}
