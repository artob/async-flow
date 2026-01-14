// This is free and unencumbered software released into the public domain.

use crate::io::SendError;
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Output<T> {
    pub sender: Sender<T>,
}

impl<T> core::fmt::Debug for Output<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OutputPort")
            .field("sender", &self.sender)
            .finish()
    }
}

impl<T> Output<T> {
    pub(crate) fn as_sender(&self) -> &Sender<T> {
        &self.sender
    }

    pub(crate) fn into_sender(self) -> Sender<T> {
        self.sender
    }

    pub fn capacity(&self) -> Option<usize> {
        Some(self.sender.capacity())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        Some(self.sender.max_capacity())
    }

    pub async fn send(&self, value: T) -> Result<(), SendError> {
        Ok(self.sender.send(value).await?)
    }
}

impl<T> AsRef<Sender<T>> for Output<T> {
    fn as_ref(&self) -> &Sender<T> {
        &self.sender
    }
}

impl<T> AsMut<Sender<T>> for Output<T> {
    fn as_mut(&mut self) -> &mut Sender<T> {
        &mut self.sender
    }
}

impl<T> From<Sender<T>> for Output<T> {
    fn from(input: Sender<T>) -> Self {
        Self { sender: input }
    }
}

impl<T> From<&Sender<T>> for Output<T> {
    fn from(input: &Sender<T>) -> Self {
        Self {
            sender: input.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static> crate::io::OutputPort<T> for Output<T> {
    async fn send(&self, value: T) -> Result<(), SendError> {
        self.send(value).await
    }
}

impl<T> crate::io::Port<T> for Output<T> {
    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    fn close(&mut self) {
        // TODO
    }
}

impl<T> MaybeLabeled for Output<T> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T> MaybeNamed for Output<T> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}
