// This is free and unencumbered software released into the public domain.

/// A port's possible states (either closed, open, or connected).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortState {
    #[default]
    Closed,
    Open,
    Connected,
}

impl PortState {
    /// Checks whether the port state is currently closed.
    pub fn is_closed(&self) -> bool {
        *self == Self::Closed
    }

    /// Checks whether the port state is currently open.
    pub fn is_open(&self) -> bool {
        *self == Self::Open
    }

    /// Checks whether the port state is currently connected.
    pub fn is_connected(&self) -> bool {
        *self == Self::Connected
    }

    pub fn as_str(&self) -> &str {
        use PortState::*;
        match self {
            Closed => "closed",
            Open => "open",
            Connected => "connected",
        }
    }
}

impl AsRef<str> for PortState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
