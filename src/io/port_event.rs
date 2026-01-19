// This is free and unencumbered software released into the public domain.

/// A port's state transition events (either connect, message, or disconnect).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortEvent<T> {
    Connect,
    Message(T),
    Disconnect,
}

impl<T> PortEvent<T> {
    /// Checks whether the event is a connect event.
    pub fn is_connect(&self) -> bool {
        matches!(self, Self::Connect)
    }

    /// Checks whether the event is a message event.
    pub fn is_message(&self) -> bool {
        matches!(self, Self::Message(_))
    }

    /// Checks whether the event is a disconnect event.
    pub fn is_disconnect(&self) -> bool {
        matches!(self, Self::Disconnect)
    }

    pub fn as_str(&self) -> &str {
        use PortEvent::*;
        match self {
            Connect => "connect",
            Message(_) => "message",
            Disconnect => "disconnect",
        }
    }
}

impl<T> AsRef<str> for PortEvent<T> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
