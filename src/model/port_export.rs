// This is free and unencumbered software released into the public domain.

use crate::Cardinality;
use core::{any::TypeId, num::NonZeroUsize};

/// A typed port export that preserves an optional message-count constraint
/// and buffer capacity.
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
    /// The buffer capacity of the boundary queue, in events; `None` for one.
    /// See [`SystemDefinition::capacities`](super::SystemDefinition::capacities).
    pub capacity: Option<NonZeroUsize>,
    /// Optional checked Tokio constructor for this payload type.
    #[cfg(feature = "tokio")]
    pub channel_factory: Option<crate::tokio::ChannelFactory>,
}

impl<I> PortExport<I> {
    /// Creates an export from raw metadata; register its runtime message type separately.
    pub fn new(id: I, type_id: TypeId) -> Self {
        Self {
            id,
            type_id,
            cardinality: None,
            capacity: None,
            #[cfg(feature = "tokio")]
            channel_factory: None,
        }
    }

    /// Adds a message-count constraint to the export.
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = Some(cardinality);
        self
    }

    /// Sets the buffer capacity of the export's boundary queue, in events.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_flow::model::{Inputs, PortExport, SystemBuilder};
    /// use core::num::NonZeroUsize;
    ///
    /// let mut builder = SystemBuilder::new();
    /// let input = Inputs::<u8>::default();
    /// builder.register_input(&input);
    /// let capacity = NonZeroUsize::new(8).unwrap();
    /// builder.export_input(PortExport::from(&input).with_capacity(capacity)).unwrap();
    /// let definition = builder.build();
    /// assert_eq!(definition.capacities[&input.id().into()], capacity);
    /// ```
    pub fn with_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.capacity = Some(capacity);
        self
    }
}

impl<I> From<(I, TypeId)> for PortExport<I> {
    fn from((id, type_id): (I, TypeId)) -> Self {
        Self::new(id, type_id)
    }
}
