// This is free and unencumbered software released into the public domain.

use super::{Inputs, Outputs};
use crate::{Connection, PortEvent};
use alloc::boxed::Box;
use core::any::TypeId;
use tokio::sync::mpsc;

pub const UNLIMITED: isize = -1;
pub const ONESHOT: isize = 1;

/// A bounded Tokio transport connecting an output port to an input port.
///
/// `tx` and `rx` are the runtime port endpoints. The channel carries
/// [`PortEvent<T>`] values: message payloads of type `T` and connection-control
/// events.
///
/// Channels created by [`bounded`](Self::bounded) or [`oneshot`](Self::oneshot)
/// have connected, initially empty endpoints. No connect event is inserted.
/// See [`Inputs`] and [`Outputs`] for draining, terminal markers, sender-clone
/// lifetime, and cancellation semantics.
#[derive(Debug, Default)]
pub struct Channel<T, const N: isize = UNLIMITED> {
    pub tx: Outputs<T, N>,
    pub rx: Inputs<T, N>,
}

impl<T, const N: isize> Channel<T, N> {
    pub fn pair() -> (Channel<T, UNLIMITED>, Channel<T, UNLIMITED>) {
        (Self::bounded(1), Self::bounded(1))
    }

    /// Creates a capacity-one connection with the one-shot cardinality marker.
    ///
    /// This currently bounds buffer capacity only; it does not limit the total
    /// number of messages sent over the connection.
    pub fn oneshot() -> Channel<T, ONESHOT> {
        Channel::from(mpsc::channel(1))
    }

    /// Creates a bounded connection.
    ///
    /// Capacity counts queued events and reserved permits, including control
    /// events. It is independent of the message cardinality marker.
    ///
    /// # Panics
    ///
    /// Panics if `buffer` is zero or exceeds Tokio's supported semaphore capacity.
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

impl<T: 'static, const N: isize> Channel<T, N> {
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: isize> Channel<T, N> {
    pub fn into_inner(self) -> (Outputs<T, N>, Inputs<T, N>) {
        (self.tx, self.rx)
    }
}

impl<T, const N: isize> From<(Outputs<T, N>, Inputs<T, N>)> for Channel<T, N> {
    fn from((tx, rx): (Outputs<T, N>, Inputs<T, N>)) -> Self {
        Self { tx, rx }
    }
}

impl<T, const N: isize> From<(mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)>
    for Channel<T, N>
{
    fn from((tx, rx): (mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)) -> Self {
        Self {
            tx: Outputs::<T, N>::from(tx),
            rx: Inputs::<T, N>::from(rx),
        }
    }
}

impl<T: 'static> Connection<T> for Channel<T> {}
