// This is free and unencumbered software released into the public domain.

use crate::io::SendError;
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use flume::Sender;

#[derive(Clone)]
pub struct Outputs<T> {
    pub(crate) tx: Sender<T>,
}

impl<T> core::fmt::Debug for Outputs<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Outputs").field("tx", &self.tx).finish()
    }
}

impl<T> Outputs<T> {
    pub fn capacity(&self) -> Option<usize> {
        self.max_capacity().map(|max| max - self.tx.len())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        self.tx.capacity()
    }

    pub async fn send(&self, value: T) -> Result<(), SendError> {
        Ok(self.tx.send_async(value).await?)
    }
}

impl<T> AsRef<Sender<T>> for Outputs<T> {
    fn as_ref(&self) -> &Sender<T> {
        &self.tx
    }
}

impl<T> AsMut<Sender<T>> for Outputs<T> {
    fn as_mut(&mut self) -> &mut Sender<T> {
        &mut self.tx
    }
}

impl<T> From<Sender<T>> for Outputs<T> {
    fn from(input: Sender<T>) -> Self {
        Self { tx: input }
    }
}

impl<T> From<&Sender<T>> for Outputs<T> {
    fn from(input: &Sender<T>) -> Self {
        Self { tx: input.clone() }
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
        self.tx.is_disconnected()
    }

    fn close(&mut self) {
        drop(self.tx.downgrade())
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
