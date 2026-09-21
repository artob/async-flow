// This is free and unencumbered software released into the public domain.

use crate::Cardinality;
use core::any::TypeId;

/// A typed port export that preserves an optional message-count constraint.
///
/// Existing `(ID, TypeId)` tuples remain accepted without cardinality metadata.
#[derive(Clone, Copy, Debug)]
pub struct PortExport<I> {
    /// The port identifier.
    pub id: I,
    /// The Rust message payload type.
    pub type_id: TypeId,
    /// A declared message-count constraint, if known.
    pub cardinality: Option<Cardinality>,
}

impl<I> From<(I, TypeId)> for PortExport<I> {
    fn from((id, type_id): (I, TypeId)) -> Self {
        Self {
            id,
            type_id,
            cardinality: None,
        }
    }
}
