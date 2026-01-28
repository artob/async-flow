// This is free and unencumbered software released into the public domain.

use super::{Inputs, Outputs};
use crate::{Connection, PortEvent};
use alloc::boxed::Box;
use core::any::TypeId;
use tokio::sync::mpsc;

pub const UNLIMITED: usize = 0;
pub const ONESHOT: usize = 1;

#[derive(Debug)]
pub struct Channel<T, const N: usize = UNLIMITED> {
    pub tx: Outputs<T, N>,
    pub rx: Inputs<T, N>,
}

impl<T> Channel<T> {
    pub fn pair() -> (Channel<T, UNLIMITED>, Channel<T, UNLIMITED>) {
        (Self::bounded(1), Self::bounded(1))
    }

    /// Creates a one-shot connection.
    pub fn oneshot() -> Channel<T, ONESHOT> {
        Channel::from(mpsc::channel(1))
    }

    /// Creates a bounded connection.
    pub fn bounded(buffer: usize) -> Channel<T, UNLIMITED> {
        Channel::from(mpsc::channel(buffer))
    }

    /// Creates a bounded, type-erased connection.
    #[allow(unused)]
    pub(crate) fn bounded_boxed(
        buffer: usize,
    ) -> (
        Box<dyn crate::io::OutputPort<T> + Send>,
        Box<dyn crate::io::InputPort<T> + Send>,
    )
    where
        T: Send + Sync + 'static,
    {
        let (outputs, inputs) = Self::bounded(buffer).into_inner();
        (Box::new(outputs), Box::new(inputs))
    }
}

impl<T: 'static, const N: usize> Channel<T, N> {
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: usize> Channel<T, N> {
    pub fn into_inner(self) -> (Outputs<T, N>, Inputs<T, N>) {
        (self.tx, self.rx)
    }
}

impl<T, const N: usize> From<(Outputs<T, N>, Inputs<T, N>)> for Channel<T, N> {
    fn from((tx, rx): (Outputs<T, N>, Inputs<T, N>)) -> Self {
        Self { tx, rx }
    }
}

impl<T, const N: usize> From<(mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)>
    for Channel<T, N>
{
    fn from((tx, rx): (mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)) -> Self {
        Self {
            tx: Outputs::<T, N>::from(tx),
            rx: Inputs::<T, N>::from(rx),
        }
    }
}

impl<T> Connection<T> for Channel<T> {}
