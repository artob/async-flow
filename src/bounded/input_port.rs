// This is free and unencumbered software released into the public domain.

use crate::{InputPort, Port};
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Receiver;

pub struct BoundedInputPort<T> {
    pub receiver: Receiver<T>,
}

impl<T> core::fmt::Debug for BoundedInputPort<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BoundedInputPort")
            .field("receiver", &self.receiver)
            .finish()
    }
}

impl<T> BoundedInputPort<T> {
    pub(crate) fn as_receiver(&self) -> &Receiver<T> {
        &self.receiver
    }

    pub(crate) fn as_receiver_mut(&mut self) -> &mut Receiver<T> {
        &mut self.receiver
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

impl<T> From<Receiver<T>> for BoundedInputPort<T> {
    fn from(input: Receiver<T>) -> Self {
        Self { receiver: input }
    }
}

#[async_trait::async_trait]
impl<T: Send> InputPort<T> for BoundedInputPort<T> {
    async fn recv(&mut self) -> Option<T> {
        self.recv().await
    }
}

impl<T> Port<T> for BoundedInputPort<T> {}

impl<T> MaybeLabeled for BoundedInputPort<T> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T> MaybeNamed for BoundedInputPort<T> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}
