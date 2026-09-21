// This is free and unencumbered software released into the public domain.

use super::UNLIMITED;
use crate::{PortDirection, PortEvent, PortState, error::SendError};
use alloc::{borrow::Cow, boxed::Box};
use core::any::TypeId;
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

/// Storage for a Tokio output handle.
///
/// [`Outputs::state`] additionally observes receiver closure on a retained sender.
#[derive(Clone, Default)]
pub enum OutputPortState<T> {
    /// No sender has been attached.
    #[default]
    Unconnected,
    /// Retains a sender; its receiver may subsequently disconnect.
    Connected(Sender<PortEvent<T>>),
    /// The connection was already disconnected when the port was constructed.
    Disconnected,
    /// This output handle was explicitly closed and its sender released.
    Closed,
}

impl<T> core::fmt::Debug for OutputPortState<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use OutputPortState::*;
        match self {
            Unconnected => f.write_str("Unconnected"),
            Connected(_) => f.write_str("Connected"),
            Disconnected => f.write_str("Disconnected"),
            Closed => f.write_str("Closed"),
        }
    }
}

impl<T> Into<SendError> for &OutputPortState<T> {
    fn into(self) -> SendError {
        Into::<PortState>::into(self).into()
    }
}

impl<T> Into<PortState> for &OutputPortState<T> {
    fn into(self) -> PortState {
        use OutputPortState::*;
        match self {
            Unconnected => PortState::Unconnected,
            Connected(tx) => {
                if tx.is_closed() {
                    PortState::Disconnected
                } else {
                    PortState::Connected
                }
            },
            Disconnected => PortState::Disconnected,
            Closed => PortState::Closed,
        }
    }
}

/// A sending runtime port sharing a bounded connection with its cloned handles.
///
/// Cloning an output shares the sender; it does not copy queued messages.
/// [`close`](Self::close) and dropping an output release only that handle. Other
/// cloned handles and raw Tokio senders can continue sending. After the last
/// sender is released, the input drains buffered events before reporting EOF;
/// previously acquired permits can delay that EOF.
///
/// Input-side disconnection rejects new sends from every output handle. An
/// explicit disconnect event is a connection-wide terminal marker when the
/// input receives it, not a request to close only the sending handle. Sending
/// the marker does not synchronously close the transport: later or concurrent
/// events can be enqueued successfully and then discarded after the marker.
///
/// `AsRef`/`AsMut` provide raw channel access while this wrapper retains a
/// sender; they panic otherwise. Raw reservations follow Tokio's permit rules.
#[derive(Clone, Default)]
pub struct Outputs<T, const N: isize = UNLIMITED> {
    pub(crate) state: OutputPortState<T>,
}

impl<T: 'static, const N: isize> Outputs<T, N> {
    /// Returns the Rust type ID of message payloads sent by this output.
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: isize> core::fmt::Debug for Outputs<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Outputs").field(&self.state).finish()
    }
}

impl<T, const N: isize> Outputs<T, N> {
    /// Closes this output handle without affecting other sender handles.
    ///
    /// This is idempotent and does not enqueue a disconnect event. Buffered
    /// events remain available to the receiver even after the last sender closes.
    pub fn close(&mut self) {
        use OutputPortState::*;
        match &self.state {
            Closed => (), // idempotent
            Unconnected | Connected(_) | Disconnected => {
                self.state = Closed;
            },
        }
    }

    /// Returns [`PortDirection::Output`].
    pub fn direction(&self) -> PortDirection {
        PortDirection::Output
    }

    /// Returns this handle's state, observing receiver closure as disconnection.
    ///
    /// Explicitly closed handles remain `Closed` regardless of their peers.
    pub fn state(&self) -> PortState {
        (&self.state).into()
    }

    /// Returns remaining event-buffer capacity while a sender is retained.
    ///
    /// Queued control events and reserved permits consume capacity too. This is
    /// a snapshot and does not imply that the receiver still accepts sends.
    pub fn capacity(&self) -> Option<usize> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Some(tx.capacity()),
            _ => None,
        }
    }

    /// Returns the total event-buffer capacity while a sender is retained.
    pub fn max_capacity(&self) -> Option<usize> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Some(tx.max_capacity()),
            _ => None,
        }
    }

    /// Enqueues a message, waiting for buffer capacity when necessary.
    ///
    /// Success means the message was enqueued, not that it was received. Closing
    /// the input or receiving a preceding disconnect marker can discard it.
    ///
    /// # Cancellation
    ///
    /// Cancelling a pending send does not enqueue its message; it drops the
    /// payload and loses its place in Tokio's capacity queue.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Unconnected`] or [`SendError::Closed`] for those local
    /// states, or [`SendError::Disconnected`] when the receiver no longer accepts
    /// sends. A failed send drops its payload; the error does not retain it.
    pub async fn send(&self, message: T) -> Result<(), SendError> {
        self.send_event(PortEvent::Message(message)).await
    }

    /// Enqueues a message or control event, waiting for buffer capacity.
    ///
    /// Every event occupies one buffer slot. `Connect` is informational;
    /// `Disconnect` terminates the entire connection when received by the input.
    /// Output clones must coordinate use of that terminal marker. Closing or
    /// dropping a sender does not synthesize control events.
    ///
    /// # Errors and cancellation
    ///
    /// Has the same enqueue, error, and cancellation guarantees as [`send`](Self::send).
    pub async fn send_event(&self, event: PortEvent<T>) -> Result<(), SendError> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Ok(tx.send(event).await?),
            _ => Err((&self.state).into()),
        }
    }

    /// A placeholder for blocking message transmission.
    ///
    /// # Panics
    ///
    /// Always panics; blocking transmission is not implemented yet.
    pub fn blocking_send(&self, _message: T) -> Result<(), SendError> {
        todo!() // TODO
    }
}

impl<T, const N: isize> AsRef<Sender<PortEvent<T>>> for Outputs<T, N> {
    fn as_ref(&self) -> &Sender<PortEvent<T>> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: isize> AsMut<Sender<PortEvent<T>>> for Outputs<T, N> {
    fn as_mut(&mut self) -> &mut Sender<PortEvent<T>> {
        use OutputPortState::*;
        match self.state {
            Connected(ref mut tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: isize> From<Sender<PortEvent<T>>> for Outputs<T, N> {
    fn from(input: Sender<PortEvent<T>>) -> Self {
        use OutputPortState::*;
        Self {
            state: if input.is_closed() {
                Disconnected
            } else {
                Connected(input)
            },
        }
    }
}

impl<T, const N: isize> From<&Sender<PortEvent<T>>> for Outputs<T, N> {
    fn from(input: &Sender<PortEvent<T>>) -> Self {
        use OutputPortState::*;
        Self {
            state: if input.is_closed() {
                Disconnected
            } else {
                Connected(input.clone())
            },
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static, const N: isize> crate::io::OutputPort<T> for Outputs<T, N> {
    async fn send(&self, message: T) -> Result<(), SendError> {
        self.send(message).await
    }
}

impl<T: Send, const N: isize> crate::io::Port<T> for Outputs<T, N> {
    fn close(&mut self) {
        self.close()
    }

    fn direction(&self) -> PortDirection {
        self.direction()
    }

    fn state(&self) -> PortState {
        self.state()
    }

    fn capacity(&self) -> Option<usize> {
        self.capacity()
    }

    fn max_capacity(&self) -> Option<usize> {
        self.max_capacity()
    }
}

impl<T, const N: isize> MaybeNamed for Outputs<T, N> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T, const N: isize> MaybeLabeled for Outputs<T, N> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}
