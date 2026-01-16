// This is free and unencumbered software released into the public domain.

use crate::io::RecvError;
use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Receiver;

pub struct Inputs<T> {
    pub(crate) rx: Receiver<T>,
}

impl<T> core::fmt::Debug for Inputs<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inputs").field("rx", &self.rx).finish()
    }
}

impl<T> Inputs<T> {
    pub(crate) fn into_receiver(self) -> Receiver<T> {
        self.rx
    }

    pub fn capacity(&self) -> Option<usize> {
        Some(self.rx.capacity())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        Some(self.rx.max_capacity())
    }

    pub async fn recv_all(&mut self) -> Result<Vec<T>, RecvError> {
        let mut inputs = Vec::new();
        while let Some(input) = self.recv().await? {
            inputs.push(input);
        }
        Ok(inputs)
    }

    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        Ok(self.rx.recv().await)
    }

    pub fn recv_blocking(&mut self) -> Result<Option<T>, RecvError> {
        Ok(self.rx.blocking_recv())
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
        self.rx.is_closed()
    }

    fn close(&mut self) {
        self.rx.close()
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
