// This is free and unencumbered software released into the public domain.

use super::UNLIMITED;
use super::quota::Quota;
use crate::{Cardinality, PortDirection, PortEvent, PortState, error::RecvError};
use alloc::{borrow::Cow, boxed::Box, sync::Arc};
use core::any::TypeId;
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Receiver;

/// Storage for a Tokio input endpoint.
///
/// [`Inputs::state`] also observes transport closure and quota exhaustion;
/// this enum records which resources the port owns.
#[derive(Default)]
pub enum InputPortState<T> {
    /// No receiver has been attached.
    #[default]
    Unconnected,
    /// Retains a receiver; its senders may subsequently disconnect.
    Connected(Receiver<PortEvent<T>>),
    /// Retains a closed receiver so accepted events can be drained.
    Disconnected(Receiver<PortEvent<T>>),
    /// Reception has finished; no receiver is retained.
    ///
    /// This is reported as [`PortState::Disconnected`].
    Ended,
    /// The input was explicitly closed and its receiver released.
    Closed,
}

impl<T> core::fmt::Debug for InputPortState<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use InputPortState::*;
        match self {
            Unconnected => f.write_str("Unconnected"),
            Connected(_) => f.write_str("Connected"),
            Disconnected(_) => f.write_str("Disconnected"),
            Ended => f.write_str("Ended"),
            Closed => f.write_str("Closed"),
        }
    }
}

impl<T> Into<RecvError> for &InputPortState<T> {
    fn into(self) -> RecvError {
        Into::<PortState>::into(self).into()
    }
}

impl<T> Into<PortState> for &InputPortState<T> {
    fn into(self) -> PortState {
        use InputPortState::*;
        match self {
            Unconnected => PortState::Unconnected,
            Connected(rx) => {
                if rx.is_closed() {
                    PortState::Disconnected
                } else {
                    PortState::Connected
                }
            },
            Disconnected(_) | Ended => PortState::Disconnected,
            Closed => PortState::Closed,
        }
    }
}

/// A receiving runtime port for messages of type `T`.
///
/// # Lifecycle
///
/// - [`disconnect`](Self::disconnect) closes the receiver to new sends while
///   retaining accepted events for draining. Previously acquired Tokio permits
///   may still deliver events; EOF waits for those permits to be used or dropped.
/// - [`close`](Self::close) releases the receiver and discards its buffered
///   events immediately. Dropping the input also releases its receiver.
/// - Dropping or closing the last sender allows buffered events to drain before
///   EOF. A disconnected input may therefore still contain readable events.
/// - `Connect` is informational. [`recv`](Self::recv) filters it out, while
///   [`recv_event`](Self::recv_event) returns it. It never reopens a connection.
/// - Receiving `Disconnect` through either receive method terminates the whole
///   connection, releases the receiver, and discards events after that marker.
///   `recv_event` returns the marker once; `recv` returns EOF instead.
///
/// # Cardinality
///
/// `N` and `MIN` declare lifetime payload bounds; system preparation can install
/// a narrower effective intersection. [`cardinality`](Self::cardinality) returns
/// those effective bounds. Reaching the maximum returns the last payload and
/// ends the input without waiting for sender handles to close. Counts are shared
/// across calls to `recv`, `recv_event`, and `recv_all`; controls do not count.
///
/// Premature EOF or a disconnect marker reports
/// [`RecvError::CardinalityUnderflow`] once, instead of successful EOF or the
/// marker. Subsequent receives return `Ok(None)`. A required unconnected input
/// reports the same shortfall. Explicit `close()` is an abort and skips this check.
///
/// Successful EOF is terminal: subsequent receives also return `Ok(None)`.
/// Unconstrained unconnected inputs and explicitly closed inputs return `Ok(None)`.
/// Natural EOF, a cardinality limit, and a disconnect marker leave the observable state
/// [`PortState::Disconnected`]; explicit `close()` sets [`PortState::Closed`].
///
/// # Raw channel access
///
/// Raw conversions and `AsRef`/`AsMut` exist only for the default `Inputs<T>`
/// type. Access also checks effective runtime bounds and panics if constrained,
/// including when system preparation installed limits on a default-typed port.
/// This prevents raw receives or receiver replacement from resetting counters.
///
/// For unconstrained ports these traits expose the receiver while retained, including during
/// graceful draining and after natural EOF. They panic on unconnected or closed
/// inputs and after a disconnect marker has released the receiver. Reading or
/// replacing the receiver directly bypasses the port's control-event handling.
///
/// ```compile_fail
/// use async_flow::{PortEvent, tokio::Inputs};
/// let (_, raw) = tokio::sync::mpsc::channel::<PortEvent<u8>>(1);
/// let _ = Inputs::<u8, -1, 1>::from(raw);
/// ```
///
/// ```compile_fail
/// use async_flow::tokio::Channel;
/// let input = Channel::<u8>::oneshot().rx;
/// let _ = input.as_ref();
/// ```
pub struct Inputs<T, const N: isize = UNLIMITED, const MIN: isize = 0> {
    pub(crate) state: InputPortState<T>,
    pub(crate) quota: Arc<Quota>,
    received: usize,
}

impl<T, const N: isize, const MIN: isize> Default for Inputs<T, N, MIN> {
    fn default() -> Self {
        Self::unconnected(const { Cardinality::from_limits(N, MIN) })
    }
}

impl<T: 'static, const N: isize, const MIN: isize> Inputs<T, N, MIN> {
    /// Returns the Rust type ID of message payloads accepted by this input.
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const N: isize, const MIN: isize> core::fmt::Debug for Inputs<T, N, MIN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Inputs").field(&self.state).finish()
    }
}

impl<T, const N: isize, const MIN: isize> Inputs<T, N, MIN> {
    pub(crate) fn unconnected(bounds: Cardinality) -> Self {
        let declared = const { Cardinality::from_limits(N, MIN) };
        assert_eq!(declared.intersection(bounds), Some(bounds));
        Self {
            state: InputPortState::Unconnected,
            quota: Quota::new(bounds),
            received: 0,
        }
    }

    pub(crate) fn with_receiver(rx: Receiver<PortEvent<T>>, quota: Arc<Quota>) -> Self {
        let state = if quota.bounds.max() == Some(0) {
            InputPortState::Ended
        } else if rx.is_closed() {
            InputPortState::Disconnected(rx)
        } else {
            InputPortState::Connected(rx)
        };
        Self {
            state,
            quota,
            received: 0,
        }
    }

    /// Returns effective lifetime message-count bounds, including negotiated limits.
    pub fn cardinality(&self) -> Cardinality {
        self.quota.bounds
    }

    fn finish(&mut self) -> Result<(), RecvError> {
        self.state = InputPortState::Ended;
        let minimum = self.quota.bounds.min();
        if self.received < minimum {
            Err(RecvError::CardinalityUnderflow {
                minimum,
                received: self.received,
            })
        } else {
            Ok(())
        }
    }

    /// Closes this input and discards all buffered events immediately.
    ///
    /// All senders observe a disconnected receiver. Repeated calls are harmless;
    /// this port cannot be reopened by a control event.
    pub fn close(&mut self) {
        use InputPortState::*;
        match self.state {
            Unconnected => self.state = Closed,
            Connected(ref mut rx) => {
                if !rx.is_closed() {
                    rx.close()
                }
                self.state = Closed;
            },
            Disconnected(_) | Ended => self.state = Closed,
            Closed => (), // idempotent
        }
    }

    /// Stops new sends while retaining accepted events for draining.
    ///
    /// Previously acquired Tokio permits can still deliver events. Receiving
    /// waits for such permits before reporting EOF. A queued disconnect marker
    /// remains terminal and discards any events following it.
    ///
    /// This is idempotent and leaves unconnected, ended, or closed inputs in
    /// their current states.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_flow::{SendError, tokio::Channel};
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> async_flow::Result {
    /// let (output, mut input) = Channel::<u8>::bounded(1).into_inner();
    /// output.send(7).await?;
    /// input.disconnect();
    /// assert_eq!(output.send(8).await, Err(SendError::Disconnected));
    /// assert_eq!(input.recv().await?, Some(7));
    /// assert_eq!(input.recv().await?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn disconnect(&mut self) {
        use InputPortState::*;
        replace_with::replace_with_or_abort(&mut self.state, |self_| match self_ {
            Unconnected => Unconnected,
            Connected(mut rx) => {
                if !rx.is_closed() {
                    rx.close()
                }
                Disconnected(rx)
            },
            Disconnected(rx) => Disconnected(rx),
            Ended => Ended,
            Closed => Closed,
        })
    }

    /// Returns [`PortDirection::Input`].
    pub fn direction(&self) -> PortDirection {
        PortDirection::Input
    }

    /// Returns the endpoint state, observing closure or exhausted quota as disconnection.
    ///
    /// `Disconnected` does not imply an empty buffer or that EOF is ready.
    pub fn state(&self) -> PortState {
        if matches!(self.state, InputPortState::Connected(_)) && self.quota.check().is_err() {
            return PortState::Disconnected;
        }
        (&self.state).into()
    }

    /// Reports whether there are currently no buffered events, including controls.
    ///
    /// This snapshot does not indicate EOF: senders or outstanding permits may
    /// still deliver events.
    pub fn is_empty(&self) -> bool {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => rx.is_empty(),
            _ => true,
        }
    }

    /// Returns remaining event-buffer capacity while a receiver is retained.
    ///
    /// Reserved permits also consume capacity. A capacity value on a disconnected
    /// input does not mean new sends are accepted. Returns `None` when no receiver
    /// is retained, including after a terminal marker or cardinality termination.
    pub fn capacity(&self) -> Option<usize> {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => Some(rx.capacity()),
            _ => None,
        }
    }

    /// Returns the total event-buffer capacity while a receiver is retained.
    pub fn max_capacity(&self) -> Option<usize> {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => Some(rx.max_capacity()),
            _ => None,
        }
    }

    /// Receives the next message, filtering informational connect events.
    ///
    /// A disconnect marker is consumed as terminal EOF; subsequent events are
    /// discarded. See the type-level lifecycle contract for other EOF conditions.
    ///
    /// # Cancellation
    ///
    /// Cancelling a pending receive does not consume a message payload. It may
    /// already have filtered connect events before waiting for a message.
    ///
    /// # Errors
    ///
    /// Reports [`RecvError::CardinalityUnderflow`] once if the stream ends below
    /// its effective minimum. Explicitly closed inputs return `Ok(None)` instead.
    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        loop {
            return match self.recv_event().await? {
                Some(PortEvent::Message(m)) => Ok(Some(m)),
                Some(PortEvent::Connect) => continue,
                Some(PortEvent::Disconnect) => Ok(None),
                None => Ok(None),
            };
        }
    }

    /// Receives the next event in enqueue order, including control events.
    ///
    /// Connect events do not change port state. A disconnect event is returned
    /// once unless it violates the minimum cardinality. It ends reception,
    /// releases the receiver, and discards queued trailing events. Later calls
    /// return `Ok(None)`. Endpoint construction and dropping synthesize no events.
    ///
    /// # Cancellation
    ///
    /// Cancelling a pending receive does not consume an event.
    ///
    /// # Errors
    ///
    /// Reports [`RecvError::CardinalityUnderflow`] once on premature termination,
    /// including a disconnect marker or a required unconnected input. The marker
    /// is consumed and trailing events discarded even when a shortfall is reported.
    pub async fn recv_event(&mut self) -> Result<Option<PortEvent<T>>, RecvError> {
        use InputPortState::*;
        let event = match self.state {
            Connected(ref mut rx) | Disconnected(ref mut rx) => rx.recv().await,
            Unconnected if self.quota.bounds.min() > 0 => {
                self.finish()?;
                return Ok(None);
            },
            _ => return Ok(None),
        };
        match &event {
            Some(PortEvent::Message(_)) => {
                // Unbounded streams only need to remember progress up to MIN.
                if self.quota.bounds.max().is_some() || self.received < self.quota.bounds.min() {
                    self.received += 1;
                }
                if self.quota.bounds.max() == Some(self.received) {
                    self.finish()?;
                }
            },
            Some(PortEvent::Disconnect) => self.finish()?,
            None if !self.quota.bounds.is_unconstrained() => self.finish()?,
            _ => (),
        }
        Ok(event)
    }

    /// A placeholder for blocking message reception.
    ///
    /// # Panics
    ///
    /// Always panics; blocking reception is not implemented yet.
    pub fn blocking_recv(&mut self) -> Result<Option<T>, RecvError> {
        todo!() // TODO
    }
}

impl<T> AsRef<Receiver<PortEvent<T>>> for Inputs<T> {
    fn as_ref(&self) -> &Receiver<PortEvent<T>> {
        assert!(
            self.quota.bounds.is_unconstrained(),
            "raw access is disabled for constrained ports"
        );
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => rx,
            _ => unreachable!(),
        }
    }
}

impl<T> AsMut<Receiver<PortEvent<T>>> for Inputs<T> {
    fn as_mut(&mut self) -> &mut Receiver<PortEvent<T>> {
        assert!(
            self.quota.bounds.is_unconstrained(),
            "raw access is disabled for constrained ports"
        );
        use InputPortState::*;
        match self.state {
            Connected(ref mut rx) | Disconnected(ref mut rx) => rx,
            _ => unreachable!(),
        }
    }
}

impl<T> From<Receiver<PortEvent<T>>> for Inputs<T> {
    fn from(input: Receiver<PortEvent<T>>) -> Self {
        Self::with_receiver(input, Quota::new(Cardinality::UNLIMITED))
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static, const N: isize, const MIN: isize> crate::io::InputPort<T>
    for Inputs<T, N, MIN>
{
    fn disconnect(&mut self) {
        self.disconnect()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        self.recv().await
    }
}

impl<T: Send, const N: isize, const MIN: isize> crate::io::Port<T> for Inputs<T, N, MIN> {
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

impl<T, const N: isize, const MIN: isize> MaybeNamed for Inputs<T, N, MIN> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T, const N: isize, const MIN: isize> MaybeLabeled for Inputs<T, N, MIN> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}
