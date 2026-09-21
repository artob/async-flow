// This is free and unencumbered software released into the public domain.

use crate::model::{PortId, SystemValidationError};
use alloc::string::String;
use core::any::TypeId;
use thiserror::Error;

/// A failure to validate or prepare a system definition for Tokio.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SystemPrepareError {
    /// The system definition's ports, exports, connections, or types are invalid.
    #[error("{0}")]
    InvalidDefinition(#[from] SystemValidationError),

    /// A payload type has no registered runtime constructor.
    #[error("no Tokio channel constructor registered for {0:?}")]
    MissingChannelFactory(TypeId),
    /// A constructor was stored under another payload type's key.
    #[error("channel constructor does not match registry key {0:?}")]
    InvalidChannelFactory(TypeId),
    /// An exported boundary port is already connected internally.
    #[error("exported port has an internal connection: {0}")]
    ConnectedExport(PortId),
    /// A block has metadata but no executable factory.
    #[error("block {index} ({name}) has no process factory")]
    MissingProcessFactory {
        /// The block's position in this definition snapshot.
        index: usize,
        /// The block's name.
        name: String,
    },
    /// A block factory failed to bind its ports or configuration.
    #[error("block {index} ({name}): {error}")]
    BlockBinding {
        /// The block's position in this definition snapshot.
        index: usize,
        /// The block's name.
        name: String,
        /// The binding failure.
        #[source]
        error: super::PortBindingError,
    },
    /// A producer's projected limits could not be represented.
    #[error("unrepresentable fan-in cardinality")]
    FanInCardinalityOverflow,
}
