// This is free and unencumbered software released into the public domain.

use crate::{PortDirection, PortEvent, PortState, error::RecvError};
use alloc::{borrow::Cow, boxed::Box};
use dogma::{MaybeLabeled, MaybeNamed};
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Default)]
pub enum InputPortState<T> {
    #[default]
    Unconnected,
    Connected(Receiver<PortEvent<T>>),
    Disconnected(Receiver<PortEvent<T>>),
    Closed,
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
            }
            Disconnected(_) => PortState::Disconnected,
            Closed => PortState::Closed,
        }
    }
}

#[derive(Default)]
pub struct Inputs<T, const N: usize = 0> {
    pub(crate) state: InputPortState<T>,
}

impl<T, const N: usize> core::fmt::Debug for Inputs<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inputs")
            //.field("state", &self.state) // TODO
            .finish()
    }
}

impl<T, const N: usize> Inputs<T, N> {
    pub fn close(&mut self) {
        use InputPortState::*;
        match self.state {
            Unconnected => self.state = Closed,
            Connected(ref mut rx) => {
                if !rx.is_closed() {
                    rx.close()
                }
                self.state = Closed;
            }
            Disconnected(_) => self.state = Closed,
            Closed => (), // idempotent
        }
    }

    pub fn disconnect(&mut self) {
        use InputPortState::*;
        replace_with::replace_with_or_abort(&mut self.state, |self_| match self_ {
            Unconnected => Unconnected,
            Connected(mut rx) => {
                if !rx.is_closed() {
                    rx.close()
                }
                Disconnected(rx)
            }
            Disconnected(rx) => Disconnected(rx),
            Closed => Closed,
        })
    }

    pub fn direction(&self) -> PortDirection {
        PortDirection::Input
    }

    pub fn state(&self) -> PortState {
        (&self.state).into()
    }

    pub fn is_empty(&self) -> bool {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => rx.is_empty(),
            _ => true,
        }
    }

    pub fn capacity(&self) -> Option<usize> {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => Some(rx.capacity()),
            _ => None,
        }
    }

    pub fn max_capacity(&self) -> Option<usize> {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => Some(rx.max_capacity()),
            _ => None,
        }
    }

    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        loop {
            return match self.recv_event().await? {
                Some(PortEvent::Message(m)) => Ok(Some(m)),
                Some(PortEvent::Connect) => continue, // TODO
                Some(PortEvent::Disconnect) => Ok(None),
                None => Ok(None),
            };
        }
    }

    pub async fn recv_event(&mut self) -> Result<Option<PortEvent<T>>, RecvError> {
        use InputPortState::*;
        match self.state {
            Connected(ref mut rx) | Disconnected(ref mut rx) => Ok(rx.recv().await),
            _ => Ok(None),
        }
    }

    pub fn blocking_recv(&mut self) -> Result<Option<T>, RecvError> {
        todo!() // TODO
    }
}

impl<T, const N: usize> AsRef<Receiver<PortEvent<T>>> for Inputs<T, N> {
    fn as_ref(&self) -> &Receiver<PortEvent<T>> {
        use InputPortState::*;
        match self.state {
            Connected(ref rx) | Disconnected(ref rx) => rx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: usize> AsMut<Receiver<PortEvent<T>>> for Inputs<T, N> {
    fn as_mut(&mut self) -> &mut Receiver<PortEvent<T>> {
        use InputPortState::*;
        match self.state {
            Connected(ref mut rx) | Disconnected(ref mut rx) => rx,
            _ => unreachable!(),
        }
    }
}

impl<T, const N: usize> From<Receiver<PortEvent<T>>> for Inputs<T, N> {
    fn from(input: Receiver<PortEvent<T>>) -> Self {
        use InputPortState::*;
        Self {
            state: if input.is_closed() {
                Disconnected(input)
            } else {
                Connected(input)
            },
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static, const N: usize> crate::io::InputPort<T> for Inputs<T, N> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        self.recv().await
    }
}

impl<T: Send, const N: usize> crate::io::Port<T> for Inputs<T, N> {
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

impl<T, const N: usize> MaybeNamed for Inputs<T, N> {
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl<T, const N: usize> MaybeLabeled for Inputs<T, N> {
    fn label(&self) -> Option<Cow<'_, str>> {
        None
    }
}
