// This is free and unencumbered software released into the public domain.

use crate::io::{Port, RecvError};
use alloc::boxed::Box;

#[async_trait::async_trait]
pub trait InputPort<T>: Port<T> {
    /// Checks if a port is empty.
    fn is_empty(&self) -> bool;

    async fn recv(&mut self) -> Result<Option<T>, RecvError>;

    // TODO: recv_event
    // TODO: try_recv
}
