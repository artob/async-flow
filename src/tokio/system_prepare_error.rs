// This is free and unencumbered software released into the public domain.

use crate::model::{InputPortId, PortId, SystemValidationError};
use core::any::TypeId;
use thiserror::Error;

/// A failure to validate or prepare a system definition for Tokio.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SystemPrepareError {
    /// The system definition's ports, exports, connections, or types are invalid.
    #[error("{0}")]
    InvalidDefinition(#[from] SystemValidationError),

    /// Multiple producers target this input; fan-in preparation is unfinished.
    #[error("fan-in preparation is not supported for input port {0}")]
    UnsupportedFanIn(InputPortId),

    /// System preparation cannot construct a channel for this message type.
    ///
    /// It currently supports only the concrete [`crate::Message`] alias;
    /// generic runtime ports can still carry other message types.
    #[error("unsupported channel type {type_id:?} for port {port}")]
    UnsupportedMessageType {
        /// The output requiring the unsupported channel type.
        port: PortId,
        /// The declared connection message type.
        type_id: TypeId,
    },
}
