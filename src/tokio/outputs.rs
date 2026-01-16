// This is free and unencumbered software released into the public domain.

use crate::io::SendError;
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Outputs<T> {
    pub(crate) tx: Option<Sender<T>>,
}

impl<T> core::fmt::Debug for Outputs<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Outputs").field("tx", &self.tx).finish()
    }
}

impl<T> Outputs<T> {
    pub(crate) fn into_sender(self) -> Sender<T> {
        self.tx.unwrap()
    }

    pub fn capacity(&self) -> Option<usize> {
        self.tx.as_ref().map(|tx| tx.capacity())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        self.tx.as_ref().map(|tx| tx.max_capacity())
    }

    pub async fn send(&self, value: T) -> Result<(), SendError> {
        if let Some(tx) = self.tx.as_ref() {
            Ok(tx.send(value).await?)
        } else {
            Err(SendError) // TODO: SendError::Closed
        }
    }

    pub fn send_blocking(&self, value: T) -> Result<(), SendError> {
        if let Some(tx) = self.tx.as_ref() {
            Ok(tx.blocking_send(value)?)
        } else {
            Err(SendError) // TODO: SendError::Closed
        }
    }
}

impl<T> AsRef<Sender<T>> for Outputs<T> {
    fn as_ref(&self) -> &Sender<T> {
        self.tx.as_ref().unwrap()
    }
}

impl<T> AsMut<Sender<T>> for Outputs<T> {
    fn as_mut(&mut self) -> &mut Sender<T> {
        self.tx.as_mut().unwrap()
    }
}

impl<T> From<Sender<T>> for Outputs<T> {
    fn from(input: Sender<T>) -> Self {
        Self { tx: Some(input) }
    }
}

impl<T> From<&Sender<T>> for Outputs<T> {
    fn from(input: &Sender<T>) -> Self {
        Self {
            tx: Some(input.clone()),
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static> crate::io::OutputPort<T> for Outputs<T> {
    async fn send(&self, value: T) -> Result<(), SendError> {
        self.send(value).await
    }
}

impl<T> crate::io::Port<T> for Outputs<T> {
    fn is_closed(&self) -> bool {
        self.tx.as_ref().map(|tx| tx.is_closed()).unwrap_or(true)
    }

    fn close(&mut self) {
        self.tx.take();
    }
}

impl<T> MaybeLabeled for Outputs<T> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T> MaybeNamed for Outputs<T> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}
