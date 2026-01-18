// This is free and unencumbered software released into the public domain.

use crate::io::RecvError;
use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Receiver;

#[derive(Default)]
pub struct Inputs<T> {
    pub(crate) rx: Option<Receiver<T>>,
}

impl<T> core::fmt::Debug for Inputs<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inputs").field("rx", &self.rx).finish()
    }
}

impl<T> Inputs<T> {
    pub fn is_open(&self) -> bool {
        !self.is_closed()
    }

    pub fn is_closed(&self) -> bool {
        self.rx.as_ref().map(|rx| rx.is_closed()).unwrap_or(true)
    }

    pub fn is_empty(&self) -> bool {
        self.rx.as_ref().map(|rx| rx.is_empty()).unwrap_or(true)
    }

    pub fn capacity(&self) -> Option<usize> {
        self.rx.as_ref().map(|rx| rx.capacity())
    }

    pub fn max_capacity(&self) -> Option<usize> {
        self.rx.as_ref().map(|rx| rx.max_capacity())
    }

    pub fn close(&mut self) {
        if let Some(rx) = self.rx.as_mut() {
            if !rx.is_closed() {
                rx.close() // idempotent
            }
        }
    }

    pub async fn recv_all(&mut self) -> Result<Vec<T>, RecvError> {
        let mut inputs = Vec::new();
        while let Some(input) = self.recv().await? {
            inputs.push(input);
        }
        Ok(inputs)
    }

    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        if let Some(rx) = self.rx.as_mut() {
            Ok(rx.recv().await)
        } else {
            Ok(None)
        }
    }

    pub fn recv_blocking(&mut self) -> Result<Option<T>, RecvError> {
        if let Some(rx) = self.rx.as_mut() {
            Ok(rx.blocking_recv())
        } else {
            Ok(None)
        }
    }
}

impl<T> AsRef<Receiver<T>> for Inputs<T> {
    fn as_ref(&self) -> &Receiver<T> {
        self.rx.as_ref().unwrap()
    }
}

impl<T> AsMut<Receiver<T>> for Inputs<T> {
    fn as_mut(&mut self) -> &mut Receiver<T> {
        self.rx.as_mut().unwrap()
    }
}

impl<T> From<Receiver<T>> for Inputs<T> {
    fn from(input: Receiver<T>) -> Self {
        Self { rx: Some(input) }
    }
}

#[async_trait::async_trait]
impl<T: Send> crate::io::InputPort<T> for Inputs<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        self.recv().await
    }
}

impl<T> crate::io::Port<T> for Inputs<T> {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    fn close(&mut self) {
        self.close()
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
