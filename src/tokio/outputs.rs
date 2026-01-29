// This is free and unencumbered software released into the public domain.

use super::UNLIMITED;
use crate::{PortDirection, PortEvent, PortState, error::SendError};
use alloc::{borrow::Cow, boxed::Box};
use core::any::TypeId;
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

#[derive(Clone, Default)]
pub enum OutputPortState<T> {
    #[default]
    Unconnected,
    Connected(Sender<PortEvent<T>>),
    Disconnected,
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

#[derive(Clone, Default)]
pub struct Outputs<T, const N: isize = UNLIMITED> {
    pub(crate) state: OutputPortState<T>,
}

impl<T: 'static, const N: isize> Outputs<T, N> {
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
    pub fn close(&mut self) {
        use OutputPortState::*;
        match &self.state {
            Closed => (), // idempotent
            Unconnected | Connected(_) | Disconnected => {
                self.state = Closed;
            },
        }
    }

    pub fn direction(&self) -> PortDirection {
        PortDirection::Output
    }

    pub fn state(&self) -> PortState {
        (&self.state).into()
    }

    pub fn capacity(&self) -> Option<usize> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Some(tx.capacity()),
            _ => None,
        }
    }

    pub fn max_capacity(&self) -> Option<usize> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Some(tx.max_capacity()),
            _ => None,
        }
    }

    pub async fn send(&self, message: T) -> Result<(), SendError> {
        self.send_event(PortEvent::Message(message)).await
    }

    pub async fn send_event(&self, event: PortEvent<T>) -> Result<(), SendError> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => Ok(tx.send(event).await?),
            _ => Err((&self.state).into()),
        }
    }

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
