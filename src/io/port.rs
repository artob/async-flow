// This is free and unencumbered software released into the public domain.

use core::fmt::Debug;
use dogma::{MaybeLabeled, MaybeNamed};

/// The common interface for ports, whether for input or output.
pub trait Port<T>: Debug + MaybeNamed + MaybeLabeled {
    /// Checks if a port is open.
    fn is_open(&self) -> bool {
        !self.is_closed()
    }

    /// Checks if a port is closed.
    fn is_closed(&self) -> bool;

    /// Closes the port without dropping it.
    fn close(&mut self);

    /// Returns the remaining buffer capacity of the connection.
    fn capacity(&self) -> Option<usize> {
        None // unknown
    }

    /// Returns the maximum buffer capacity of the connection.
    fn max_capacity(&self) -> Option<usize> {
        None // unknown
    }
}
