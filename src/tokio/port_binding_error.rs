// This is free and unencumbered software released into the public domain.

use crate::{Cardinality, model::PortId};
use alloc::string::String;
use core::any::TypeId;
use thiserror::Error;

/// A typed port could not be bound to a process or an external caller.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PortBindingError {
    /// No such port exists in the requested binding scope.
    #[error("unknown port: {0}")]
    UnknownPort(PortId),
    /// The port belongs to another block.
    #[error("port belongs to another block: {0}")]
    ForeignPort(PortId),
    /// This endpoint has already been moved to its owner.
    #[error("port already claimed: {0}")]
    AlreadyClaimed(PortId),
    /// The port is not an exported boundary port of the requested direction.
    #[error("port is not exported: {0}")]
    NotExported(PortId),
    /// A connected block port was omitted by its process factory.
    #[error("connected port was not claimed: {0}")]
    UnclaimedPort(PortId),
    /// The requested payload type differs from the validated type.
    #[error("type mismatch for port {port}: expected {expected:?}, requested {requested:?}")]
    TypeMismatch {
        /// The requested port.
        port: PortId,
        /// The validated payload type.
        expected: TypeId,
        /// The requested payload type.
        requested: TypeId,
    },
    /// The descriptor contradicts the already installed limits.
    #[error(
        "cardinality mismatch for port {port}: effective {effective:?}, requested {requested:?}"
    )]
    CardinalityMismatch {
        /// The requested port.
        port: PortId,
        /// Effective runtime bounds.
        effective: Cardinality,
        /// The descriptor's declared bounds.
        requested: Cardinality,
    },
    /// A factory rejected its configuration before process startup.
    #[error("invalid block configuration: {0}")]
    InvalidConfiguration(String),
}
