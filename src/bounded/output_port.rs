// This is free and unencumbered software released into the public domain.

use crate::{OutputPort, Port};
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::{Sender, error::SendError};

#[derive(Clone)]
pub struct BoundedOutputPort<T> {
    pub sender: Sender<T>,
}

impl<T> core::fmt::Debug for BoundedOutputPort<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BoundedOutputPort")
            .field("sender", &self.sender)
            .finish()
    }
}

impl<T> BoundedOutputPort<T> {
    pub(crate) fn as_sender(&self) -> &Sender<T> {
        &self.sender
    }

    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.sender.send(value).await
    }
}

impl<T> From<Sender<T>> for BoundedOutputPort<T> {
    fn from(input: Sender<T>) -> Self {
        Self { sender: input }
    }
}

impl<T> From<&Sender<T>> for BoundedOutputPort<T> {
    fn from(input: &Sender<T>) -> Self {
        Self {
            sender: input.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static> OutputPort<T> for BoundedOutputPort<T> {
    async fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.send(value).await
    }
}

impl<T> Port<T> for BoundedOutputPort<T> {}

impl<T> MaybeLabeled for BoundedOutputPort<T> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T> MaybeNamed for BoundedOutputPort<T> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}
