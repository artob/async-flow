// This is free and unencumbered software released into the public domain.

use super::quota::Quota;
use super::{Inputs, Outputs};
use crate::{Cardinality, Connection, PortEvent};
use alloc::boxed::Box;
use core::any::TypeId;
use tokio::sync::mpsc;

/// An unlimited message count; buffers remain bounded.
pub const UNLIMITED: isize = -1;
/// An inclusive maximum of one message payload.
pub const ONESHOT: isize = 1;

/// A bounded Tokio transport connecting an output port to an input port.
///
/// `tx` and `rx` are the runtime port endpoints. The channel carries
/// [`PortEvent<T>`] values: message payloads of type `T` and connection-control
/// events.
///
/// `N` is the inclusive maximum payload count (`-1` means unlimited), and `MIN`
/// is the inclusive minimum. The quota is shared across all output clones.
/// Controls consume buffer capacity, not message quota. A zero maximum creates
/// an already-ended stream; otherwise new endpoints are connected and empty.
/// No connect or disconnect event is synthesized on reaching a quota.
/// See [`Inputs`] and [`Outputs`] for draining, terminal markers, sender-clone
/// lifetime, and cancellation semantics.
///
/// # Examples
///
/// ```
/// use async_flow::{SendError, tokio::Channel};
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> async_flow::Result {
/// let (output, mut input) = Channel::<u8>::oneshot().into_inner();
/// let peer = output.clone();
/// output.send(7).await?;
/// assert_eq!(peer.send(8).await, Err(SendError::CardinalityExceeded { maximum: 1 }));
/// assert_eq!(input.recv().await?, Some(7));
/// assert_eq!(input.recv().await?, None); // Both senders are still alive.
/// # Ok(())
/// # }
/// ```
///
/// Invalid const bounds cannot be constructed:
///
/// ```compile_fail
/// use async_flow::tokio::Channel;
/// let _ = Channel::<u8, 1, 2>::bounded(1);
/// ```
///
/// Constrained channels cannot be built from unguarded Tokio handles:
///
/// ```compile_fail
/// use async_flow::{PortEvent, tokio::Channel};
/// let raw = tokio::sync::mpsc::channel::<PortEvent<u8>>(1);
/// let _ = Channel::<u8, 1>::from(raw);
/// ```
#[derive(Debug)]
pub struct Channel<T, const N: isize = UNLIMITED, const MIN: isize = 0> {
    /// The sending endpoint, sharing its quota with all cloned senders.
    pub tx: Outputs<T, N, MIN>,
    /// The receiving endpoint, enforcing the stream's minimum at EOF.
    pub rx: Inputs<T, N, MIN>,
}

impl<T, const N: isize, const MIN: isize> Default for Channel<T, N, MIN> {
    fn default() -> Self {
        Self {
            tx: Outputs::default(),
            rx: Inputs::default(),
        }
    }
}

impl<T, const N: isize, const MIN: isize> Channel<T, N, MIN> {
    /// Creates two independent capacity-one connections with these cardinality bounds.
    pub fn pair() -> (Self, Self) {
        (Self::bounded(1), Self::bounded(1))
    }

    /// Creates a bounded connection.
    ///
    /// Capacity counts queued events and reserved permits, including control
    /// events. The const generics constrain message counts independently of this
    /// buffer capacity. Invalid const-generic bounds are rejected at compile time.
    ///
    /// # Panics
    ///
    /// Panics if `buffer` is zero or exceeds Tokio's supported semaphore capacity.
    pub fn bounded(buffer: usize) -> Self {
        Self::with_cardinality(buffer, const { Cardinality::from_limits(N, MIN) })
    }

    pub(crate) fn with_cardinality(buffer: usize, bounds: Cardinality) -> Self {
        let declared = const { Cardinality::from_limits(N, MIN) };
        assert_eq!(declared.intersection(bounds), Some(bounds));
        let (tx, rx) = mpsc::channel(buffer);
        let quota = Quota::new(bounds);
        Self {
            tx: Outputs::with_sender(tx, quota.clone()),
            rx: Inputs::with_receiver(rx, quota),
        }
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

impl<T> Channel<T> {
    /// Creates a capacity-one connection accepting at most one message payload.
    ///
    /// All sender clones share that single allowance. After receiving the message,
    /// the input reaches EOF even if senders remain alive. For exactly one required
    /// message, use `Channel::<T, 1, 1>::bounded(1)` instead.
    pub fn oneshot() -> Channel<T, ONESHOT> {
        Channel::<T, ONESHOT>::bounded(1)
    }
}

impl<T: 'static, const N: isize, const MIN: isize> Channel<T, N, MIN> {
    /// Returns the Rust type ID of message payloads.
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: isize, const MIN: isize> Channel<T, N, MIN> {
    /// Separates the guarded endpoints without resetting their cardinality state.
    pub fn into_inner(self) -> (Outputs<T, N, MIN>, Inputs<T, N, MIN>) {
        (self.tx, self.rx)
    }
}

impl<T, const N: isize, const MIN: isize> From<(Outputs<T, N, MIN>, Inputs<T, N, MIN>)>
    for Channel<T, N, MIN>
{
    fn from((tx, rx): (Outputs<T, N, MIN>, Inputs<T, N, MIN>)) -> Self {
        Self { tx, rx }
    }
}

impl<T> From<(mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)> for Channel<T> {
    fn from((tx, rx): (mpsc::Sender<PortEvent<T>>, mpsc::Receiver<PortEvent<T>>)) -> Self {
        Self {
            tx: Outputs::from(tx),
            rx: Inputs::from(rx),
        }
    }
}

impl<T: 'static, const N: isize, const MIN: isize> Connection<T> for Channel<T, N, MIN> {}
