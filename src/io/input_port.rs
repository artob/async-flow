// This is free and unencumbered software released into the public domain.

use crate::{error::RecvError, io::Port};
use alloc::boxed::Box;
use alloc::vec::Vec;

#[async_trait::async_trait]
pub trait InputPort<T: Send>: Port<T> {
    /// Checks if this port is empty.
    fn is_empty(&self) -> bool;

    async fn recv(&mut self) -> Result<Option<T>, RecvError>;

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
