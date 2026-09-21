// This is free and unencumbered software released into the public domain.

/// A port's possible states (either unconnected, connected, disconnected, or closed).
///
/// Disconnection and buffer exhaustion are distinct: a disconnected input may
/// still have readable events or outstanding permits. Backend APIs define the
/// precise draining and EOF semantics.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortState {
    /// No connection has been attached to the endpoint.
    #[default]
    Unconnected,

    /// The endpoint has an open transport connection.
    Connected,

    /// The connection no longer accepts ordinary sends; buffered events may remain.
    Disconnected,

    /// This endpoint was explicitly closed.
    Closed,
}

impl PortState {
    /// Checks whether the port state is currently unconnected.
    pub fn is_unconnected(&self) -> bool {
        *self == Self::Unconnected
    }

    /// Checks whether the port state is currently connected.
    pub fn is_connected(&self) -> bool {
        *self == Self::Connected
    }

    /// Checks whether the port state is currently disconnected.
    pub fn is_disconnected(&self) -> bool {
        *self == Self::Disconnected
    }

    /// Checks whether the port state is currently closed.
    pub fn is_closed(&self) -> bool {
        *self == Self::Closed
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Unconnected => "unconnected",
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Closed => "closed",
        }
    }
}

impl AsRef<str> for PortState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
