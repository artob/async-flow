// This is free and unencumbered software released into the public domain.

use super::{InputPortId, OutputPortId, PortId};
use crate::Cardinality;

/// A port ID with optional cardinality metadata for builder registration.
///
/// Raw IDs convert without a constraint. References to model port descriptors
/// preserve their const-generic bounds.
#[derive(Clone, Copy, Debug)]
pub struct PortRegistration<I> {
    /// The port identifier.
    pub id: I,
    /// A declared message-count constraint, if known.
    pub cardinality: Option<Cardinality>,
}

impl<I> From<I> for PortRegistration<I> {
    fn from(id: I) -> Self {
        Self {
            id,
            cardinality: None,
        }
    }
}

impl From<InputPortId> for PortRegistration<PortId> {
    fn from(id: InputPortId) -> Self {
        Self::from(PortId::Input(id))
    }
}

impl From<OutputPortId> for PortRegistration<PortId> {
    fn from(id: OutputPortId) -> Self {
        Self::from(PortId::Output(id))
    }
}
