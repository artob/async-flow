// This is free and unencumbered software released into the public domain.

use super::{PortDirection, PortState};
use dogma::{MaybeLabeled, MaybeNamed};

/// The common runtime interface for a block's input or output connection point.
///
/// Blocks exchange messages through ports. This interface provides optional
/// names and labels; backend implementations provide the runtime endpoints.
pub trait Port<T: Send>: MaybeNamed + MaybeLabeled {
    /// Closes this port without dropping it, returning immediately.
    ///
    /// If the port had an open connection, it will be disconnected.
    /// If the port was already closed, no further action is taken.
    /// There is no facility to reopen a port once it has been closed.
    /// Closing an output handle need not close other handles sharing its
    /// connection. Consult the backend for buffer retention and draining rules.
    fn close(&mut self);

    /// The dataflow direction of this port.
    fn direction(&self) -> PortDirection;

    /// The current state of this port.
    fn state(&self) -> PortState;

    /// Checks whether this port is an input port.
    fn is_input(&self) -> bool {
        self.direction().is_input()
    }

    /// Checks whether this port is an output port.
    fn is_output(&self) -> bool {
        self.direction().is_output()
    }

    /// Checks whether this port is currently unconnected.
    fn is_unconnected(&self) -> bool {
        self.state().is_unconnected()
    }

    /// Checks whether this port is currently connected.
    fn is_connected(&self) -> bool {
        self.state().is_connected()
    }

    /// Checks whether this port is currently disconnected.
    fn is_disconnected(&self) -> bool {
        self.state().is_disconnected()
    }

    /// Checks whether this port is currently closed.
    fn is_closed(&self) -> bool {
        self.state().is_closed()
    }

    /// Returns the remaining buffer capacity of the connection.
    fn capacity(&self) -> Option<usize> {
        None // unknown
    }

    /// Returns the maximum buffer capacity of the connection.
    fn max_capacity(&self) -> Option<usize> {
        None // unknown
    }
}

impl<T: Send> core::fmt::Debug for &dyn Port<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Port")
            .field("name", &self.name())
            .field("state", &self.state())
            .finish()
    }
}
