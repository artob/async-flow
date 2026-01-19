// This is free and unencumbered software released into the public domain.

use super::PortState;
use crate::PortDirection;
use dogma::{MaybeLabeled, MaybeNamed};

/// The common interface for ports, whether for input or output.
pub trait Port<T: Send>: MaybeNamed + MaybeLabeled {
    /// Closes this port without dropping it, returning immediately.
    ///
    /// If the port had an open connection, it will be disconnected.
    /// If the port was already closed, no further action is taken.
    /// There is no facility to reopen a port once it has been closed.
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

    /// Checks whether this port is currently closed.
    fn is_closed(&self) -> bool {
        self.state().is_closed()
    }

    /// Checks whether this port is currently open.
    fn is_open(&self) -> bool {
        self.state().is_open()
    }

    /// Checks whether this port is currently connected.
    fn is_connected(&self) -> bool {
        self.state().is_connected()
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
