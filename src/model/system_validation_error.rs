// This is free and unencumbered software released into the public domain.

use super::{InputPortId, OutputPortId, PortId};
use crate::Cardinality;
use core::any::TypeId;
use thiserror::Error;

/// An invalid system definition detected by [`super::SystemDefinition::validate`].
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SystemValidationError {
    /// Constraints on a port or its incoming stream have no common message count.
    #[error("incompatible cardinalities for port {port}: {first:?} and {second:?}")]
    IncompatibleCardinality {
        /// The constrained port.
        port: PortId,
        /// The existing or aggregate producer constraint.
        first: Cardinality,
        /// The additional or consumer constraint.
        second: Cardinality,
    },

    /// Aggregate producer bounds cannot be represented as `usize` message counts.
    #[error("cardinality overflow for input port {0}")]
    CardinalityOverflow(InputPortId),
    /// An input ID is nonnegative or an output ID is nonpositive.
    #[error("invalid port ID: {0}")]
    InvalidPortId(PortId),

    /// An export or connection references a port that has not been declared.
    #[error("unregistered port ID: {0}")]
    UnregisteredPort(PortId),

    /// A port is listed more than once within or across block definitions.
    #[error("duplicate block port ID: {0}")]
    DuplicatePort(PortId),

    /// An output is connected to more than one input.
    #[error("already connected output port ID: {0}")]
    AlreadyConnectedOutput(OutputPortId),

    /// Block metadata, exports, or connections disagree about a port's type.
    #[error("conflicting types for port {port}: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        /// The port with conflicting type declarations.
        port: PortId,
        /// The first type declared for the port during validation.
        expected: TypeId,
        /// The conflicting type encountered later during validation.
        actual: TypeId,
    },
}
