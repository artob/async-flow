// This is free and unencumbered software released into the public domain.

use crate::{PortDirection, PortEvent, PortState, error::SendError};
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug, Default)]
pub enum OutputPortState<T> {
    #[default]
    Unconnected,
    Connected(Sender<PortEvent<T>>),
    Disconnected,
    Closed,
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
            }
            Disconnected => PortState::Disconnected,
            Closed => PortState::Closed,
        }
    }
}

#[derive(Clone, Default)]
pub struct Outputs<T, const N: usize = 0> {
    pub(crate) state: OutputPortState<T>,
}

impl<T, const N: usize> core::fmt::Debug for Outputs<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Outputs")
            //.field("state", &self.state) // TODO
            .finish()
    }
}

impl<T, const N: usize> Outputs<T, N> {
    pub fn close(&mut self) {
        use OutputPortState::*;
        match &self.state {
            Closed => (), // idempotent
            Unconnected | Connected(_) | Disconnected => {
                self.state = Closed;
            }
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
            _ => Err(SendError), // TODO: SendError::Closed
        }
    }

    pub fn send_blocking(&self, _message: T) -> Result<(), SendError> {
        todo!() // TODO
    }
}

impl<T, const N: usize> AsRef<Sender<PortEvent<T>>> for Outputs<T, N> {
    fn as_ref(&self) -> &Sender<PortEvent<T>> {
        use OutputPortState::*;
        match self.state {
            Connected(ref tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: usize> AsMut<Sender<PortEvent<T>>> for Outputs<T, N> {
    fn as_mut(&mut self) -> &mut Sender<PortEvent<T>> {
        use OutputPortState::*;
        match self.state {
            Connected(ref mut tx) => tx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: usize> From<Sender<PortEvent<T>>> for Outputs<T, N> {
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

impl<T, const N: usize> From<&Sender<PortEvent<T>>> for Outputs<T, N> {
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
impl<T: Send + 'static, const N: usize> crate::io::OutputPort<T> for Outputs<T, N> {
    async fn send(&self, message: T) -> Result<(), SendError> {
        self.send(message).await
    }
}

impl<T: Send, const N: usize> crate::io::Port<T> for Outputs<T, N> {
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

impl<T, const N: usize> MaybeNamed for Outputs<T, N> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T, const N: usize> MaybeLabeled for Outputs<T, N> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}
