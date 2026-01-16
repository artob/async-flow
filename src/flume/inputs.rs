// This is free and unencumbered software released into the public domain.

use crate::io::RecvError;
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use flume::Receiver;

#[derive(Clone)]
pub struct Inputs<T> {
    pub(crate) rx: Receiver<T>,
}

impl<T> core::fmt::Debug for Inputs<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inputs").field("rx", &self.rx).finish()
    }
}

impl<T> Inputs<T> {
    pub fn capacity(&self) -> Option<usize> {
        self.max_capacity().map(|max| max - self.rx.len())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        self.rx.capacity()
    }

    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        match self.rx.recv_async().await {
            Ok(value) => Ok(Some(value)),
            Err(flume::RecvError::Disconnected) => Ok(None),
        }
    }
}

impl<T> AsRef<Receiver<T>> for Inputs<T> {
    fn as_ref(&self) -> &Receiver<T> {
        &self.rx
    }
}

impl<T> AsMut<Receiver<T>> for Inputs<T> {
    fn as_mut(&mut self) -> &mut Receiver<T> {
        &mut self.rx
    }
}

impl<T> From<Receiver<T>> for Inputs<T> {
    fn from(input: Receiver<T>) -> Self {
        Self { rx: input }
    }
}

impl<T> From<&Receiver<T>> for Inputs<T> {
    fn from(input: &Receiver<T>) -> Self {
        Self { rx: input.clone() }
    }
}

#[async_trait::async_trait]
impl<T: Send> crate::io::InputPort<T> for Inputs<T> {
    fn is_empty(&self) -> bool {
        self.rx.is_empty()
    }

    async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        self.recv().await
    }
}

impl<T> crate::io::Port<T> for Inputs<T> {
    fn is_closed(&self) -> bool {
        self.rx.is_disconnected()
    }

    fn close(&mut self) {
        // TODO
    }
}

impl<T> MaybeLabeled for Inputs<T> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T> MaybeNamed for Inputs<T> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}
