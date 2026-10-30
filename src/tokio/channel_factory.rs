// This is free and unencumbered software released into the public domain.

use super::{Channel, SystemPrepareError, runtime_ports::ErasedPort};
use crate::{Cardinality, model::OutputPortId};
use alloc::vec::Vec;
use core::any::TypeId;

pub(crate) struct FanInConnection {
    pub(crate) input: ErasedPort,
    pub(crate) outputs: Vec<(OutputPortId, Cardinality, ErasedPort)>,
}

/// Builds a fan-in connection: the input's bounds, and each producer's ID,
/// bounds and queue capacity.
type FanInFactory = fn(
    Cardinality,
    &[(OutputPortId, Cardinality, usize)],
) -> Result<FanInConnection, SystemPrepareError>;

/// A checked type-erased constructor for Tokio connections carrying one Rust type.
///
/// Typed builder connections and exports register this automatically. Use
/// `SystemBuilder::register_message_type` when editing raw `TypeId` metadata.
#[derive(Clone, Copy)]
pub struct ChannelFactory {
    type_id: TypeId,
    single: fn(usize, Cardinality) -> (ErasedPort, ErasedPort),
    fan_in: FanInFactory,
}

impl ChannelFactory {
    /// Creates a constructor for `T` without requiring `Clone`, `Default`, or `Sync`.
    pub fn of<T: Send + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            single: |buffer, bounds| {
                let (output, input) = Channel::<T>::with_cardinality(buffer, bounds).into_inner();
                (
                    alloc::boxed::Box::new(output),
                    alloc::boxed::Box::new(input),
                )
            },
            fan_in: super::merged_inputs::connect::<T>,
        }
    }

    /// Returns the payload type this constructor actually creates.
    pub fn message_type(&self) -> TypeId {
        self.type_id
    }

    pub(crate) fn single(&self, buffer: usize, bounds: Cardinality) -> (ErasedPort, ErasedPort) {
        (self.single)(buffer, bounds)
    }

    pub(crate) fn fan_in(
        &self,
        bounds: Cardinality,
        sources: &[(OutputPortId, Cardinality, usize)],
    ) -> Result<FanInConnection, SystemPrepareError> {
        (self.fan_in)(bounds, sources)
    }
}

impl core::fmt::Debug for ChannelFactory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChannelFactory")
            .field("message_type", &self.type_id)
            .finish()
    }
}
