// This is free and unencumbered software released into the public domain.

/// An event carried by a connection: a message or a connection-control event.
///
/// `T` is the message payload; connect and disconnect events carry control
/// information. Endpoint APIs determine how these events affect port state.
///
/// In the Tokio backend, `Connect` is informational. Receiving `Disconnect`
/// ends the connection, discards subsequent buffered events, and makes future
/// receives return EOF. Sending that marker does not synchronously close the
/// transport. Endpoint construction and dropping do not synthesize either event.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortEvent<T> {
    /// Signals a connection event.
    Connect,
    /// Carries one message of type `T`.
    Message(T),
    /// Signals a disconnection event.
    Disconnect,
}

impl<T> PortEvent<T> {
    pub fn message(&self) -> Option<&T> {
        match self {
            Self::Message(message) => Some(message),
            _ => None,
        }
    }

    pub fn into_message(self) -> Option<T> {
        match self {
            Self::Message(message) => Some(message),
            _ => None,
        }
    }

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
        match self {
            Self::Connect => "connect",
            Self::Message(_) => "message",
            Self::Disconnect => "disconnect",
        }
    }
}

impl<T> AsRef<str> for PortEvent<T> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
