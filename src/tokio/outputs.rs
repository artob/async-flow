// This is free and unencumbered software released into the public domain.

use super::UNLIMITED;
use super::quota::Quota;
use crate::{Cardinality, PortDirection, PortEvent, PortState, error::SendError};
use alloc::{borrow::Cow, boxed::Box, sync::Arc};
use core::{any::TypeId, future::poll_fn, pin::pin, task::Poll};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

/// Storage for a Tokio output handle.
///
/// [`Outputs::state`] additionally observes receiver closure and shared quota exhaustion.
#[derive(Default)]
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

impl<T> Clone for OutputPortState<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Unconnected => Self::Unconnected,
            Self::Connected(tx) => Self::Connected(tx.clone()),
            Self::Disconnected => Self::Disconnected,
            Self::Closed => Self::Closed,
        }
    }
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
/// sender is released, the input drains buffered events and checks its minimum before EOF;
/// previously acquired permits can delay that EOF.
///
/// Input-side disconnection rejects new sends from every output handle. An
/// explicit disconnect event is a connection-wide terminal marker when the
/// input receives it, not a request to close only the sending handle. Sending
/// the marker does not synchronously close the transport: later or concurrent
/// events can be enqueued successfully and then discarded after the marker.
///
/// # Cardinality
///
/// `N` and `MIN` declare lifetime payload bounds. Effective constraints may be
/// narrowed during system preparation and are available through
/// [`cardinality`](Self::cardinality). Every cloned sender shares the same quota,
/// even for non-`Clone` payloads. Failed or cancelled pending sends spend no quota.
/// The last allowed payload stops further sends and wakes quota/capacity waiters;
/// the input drains accepted payloads and ends even while sender handles survive.
/// Controls do not spend quota. Sends started after exhaustion fail; concurrent
/// in-flight controls may still be queued and are discarded after the last payload.
///
/// # Raw channel access
///
/// Raw conversions and `AsRef`/`AsMut` exist only for the default `Outputs<T>`
/// type. Access additionally checks effective bounds and panics if constrained,
/// including limits installed by system preparation. This prevents raw sender
/// cloning or replacement from bypassing or resetting a quota.
/// For unconstrained ports, access requires a retained sender and otherwise
/// panics. Raw reservations follow Tokio's permit rules.
///
/// ```compile_fail
/// use async_flow::tokio::Channel;
/// let output = Channel::<u8>::oneshot().tx;
/// let _ = output.as_ref();
/// ```
///
/// ```compile_fail
/// use async_flow::{PortEvent, tokio::Outputs};
/// let (raw, _) = tokio::sync::mpsc::channel::<PortEvent<u8>>(1);
/// let _ = Outputs::<u8, 1>::from(raw);
/// ```
pub struct Outputs<T, const N: isize = UNLIMITED, const MIN: isize = 0> {
    pub(crate) state: OutputPortState<T>,
    pub(crate) quota: Arc<Quota>,
}

impl<T, const N: isize, const MIN: isize> Default for Outputs<T, N, MIN> {
    fn default() -> Self {
        Self::unconnected(const { Cardinality::from_limits(N, MIN) })
    }
}

impl<T, const N: isize, const MIN: isize> Clone for Outputs<T, N, MIN> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            quota: self.quota.clone(),
        }
    }
}

impl<T: 'static, const N: isize, const MIN: isize> Outputs<T, N, MIN> {
    /// Returns the Rust type ID of message payloads sent by this output.
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: isize, const MIN: isize> core::fmt::Debug for Outputs<T, N, MIN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Outputs").field(&self.state).finish()
    }
}

impl<T, const N: isize, const MIN: isize> Outputs<T, N, MIN> {
    pub(crate) fn unconnected(bounds: Cardinality) -> Self {
        let declared = const { Cardinality::from_limits(N, MIN) };
        assert_eq!(declared.intersection(bounds), Some(bounds));
        Self {
            state: OutputPortState::Unconnected,
            quota: Quota::new(bounds),
        }
    }

    pub(crate) fn with_sender(tx: Sender<PortEvent<T>>, quota: Arc<Quota>) -> Self {
        let state = if tx.is_closed() {
            OutputPortState::Disconnected
        } else {
            OutputPortState::Connected(tx)
        };
        Self { state, quota }
    }

    /// Returns effective lifetime message-count bounds, shared by cloned senders.
    pub fn cardinality(&self) -> Cardinality {
        self.quota.bounds
    }

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

    /// Returns this handle's state, observing closure or exhausted quota as disconnection.
    ///
    /// Explicitly closed handles remain `Closed` regardless of their peers.
    pub fn state(&self) -> PortState {
        if matches!(self.state, OutputPortState::Connected(_)) && self.quota.check().is_err() {
            return PortState::Disconnected;
        }
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
    /// Once the shared maximum is reached, further sends return
    /// [`SendError::CardinalityExceeded`] without waiting for buffer space.
    /// Explicitly closed handles still return `Closed`.
    pub async fn send(&self, message: T) -> Result<(), SendError> {
        self.send_event(PortEvent::Message(message)).await
    }

    /// Enqueues a message or control event, waiting for buffer capacity.
    ///
    /// Every event occupies one buffer slot. `Connect` is informational;
    /// `Disconnect` terminates the entire connection when received by the input.
    /// Output clones must coordinate use of that terminal marker. Closing or
    /// dropping a sender does not synthesize control events.
    /// No marker is needed after the final allowed payload: cardinality exhaustion
    /// already ends the stream once that payload is received.
    ///
    /// # Errors and cancellation
    ///
    /// Has the same enqueue, error, and cancellation guarantees as [`send`](Self::send).
    pub async fn send_event(&self, event: PortEvent<T>) -> Result<(), SendError> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => {
                self.quota.check()?;
                if self.quota.bounds.max().is_none() {
                    return Ok(tx.send(event).await?);
                }
                // Poll quota exhaustion first without using a std-only macro.
                // Both race futures are dropped before committing the payload.
                let permit = {
                    let mut exhausted = pin!(self.quota.wait_exhausted());
                    let mut reserving = pin!(tx.reserve());
                    poll_fn(|cx| {
                        if let Poll::Ready(error) = exhausted.as_mut().poll(cx) {
                            return Poll::Ready(Err(error));
                        }
                        reserving
                            .as_mut()
                            .poll(cx)
                            .map(|result| result.map_err(SendError::from))
                    })
                    .await?
                };
                // No await after quota commitment: cancellation cannot spend a
                // message allowance without enqueueing its payload.
                if event.is_message() {
                    self.quota.commit()?;
                } else {
                    self.quota.check()?;
                }
                permit.send(event);
                self.quota.notify_if_exhausted();
                Ok(())
            },
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

impl<T> AsRef<Sender<PortEvent<T>>> for Outputs<T> {
    fn as_ref(&self) -> &Sender<PortEvent<T>> {
        assert!(
            self.quota.bounds.is_unconstrained(),
            "raw access is disabled for constrained ports"
        );
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T> AsMut<Sender<PortEvent<T>>> for Outputs<T> {
    fn as_mut(&mut self) -> &mut Sender<PortEvent<T>> {
        assert!(
            self.quota.bounds.is_unconstrained(),
            "raw access is disabled for constrained ports"
        );
        use OutputPortState::*;
        match self.state {
            Connected(ref mut tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T> From<Sender<PortEvent<T>>> for Outputs<T> {
    fn from(input: Sender<PortEvent<T>>) -> Self {
        Self::with_sender(input, Quota::new(Cardinality::UNLIMITED))
    }
}

impl<T> From<&Sender<PortEvent<T>>> for Outputs<T> {
    fn from(input: &Sender<PortEvent<T>>) -> Self {
        Self::from(input.clone())
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static, const N: isize, const MIN: isize> crate::io::OutputPort<T>
    for Outputs<T, N, MIN>
{
    async fn send(&self, message: T) -> Result<(), SendError> {
        self.send(message).await
    }
}

impl<T: Send, const N: isize, const MIN: isize> crate::io::Port<T> for Outputs<T, N, MIN> {
    fn cardinality(&self) -> Option<Cardinality> {
        Some(self.cardinality())
    }

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

impl<T, const N: isize, const MIN: isize> MaybeNamed for Outputs<T, N, MIN> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T, const N: isize, const MIN: isize> MaybeLabeled for Outputs<T, N, MIN> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}
