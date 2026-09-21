// This is free and unencumbered software released into the public domain.

use super::{InputPortId, OutputPortId};
use alloc::vec::Vec;
use core::any::TypeId;

/// The name of a block.
pub use dogma::Named as BlockName;

/// The port and message-type metadata of a block.
///
/// A block is an encapsulated system component that processes messages through
/// ports. This trait describes its interfaces for a system definition; it does
/// not supply the executable body of a block process.
pub trait BlockDefinition: BlockName {
    /// Returns this block's input IDs, each listed once.
    fn inputs(&self) -> Vec<InputPortId> {
        Vec::new()
    }

    /// Returns this block's output IDs, each listed once.
    fn outputs(&self) -> Vec<OutputPortId> {
        Vec::new()
    }

    /// Returns the declared message type of an input, if known.
    ///
    /// Validation checks exports and connections against this type. The default
    /// `None` leaves the type to be inferred from exports or connections.
    fn input_type(&self, _input: InputPortId) -> Option<TypeId> {
        None
    }

    /// Returns the declared message type of an output, if known.
    ///
    /// Validation checks exports and connections against this type. The default
    /// `None` leaves the type to be inferred from exports or connections.
    fn output_type(&self, _output: OutputPortId) -> Option<TypeId> {
        None
    }
}
