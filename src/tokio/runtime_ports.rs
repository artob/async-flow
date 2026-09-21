// This is free and unencumbered software released into the public domain.

use super::{Inputs, Outputs, PortBindingError};
use crate::{
    Cardinality,
    model::{InputPortId, OutputPortId, PortId},
};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
};
use core::any::{Any, TypeId};

pub(crate) type ErasedPort = Box<dyn Any + Send>;

pub(crate) struct PortSlot {
    pub(crate) type_id: Option<TypeId>,
    pub(crate) bounds: Cardinality,
    pub(crate) endpoint: Option<ErasedPort>,
    pub(crate) connected: bool,
    pub(crate) claimed: bool,
}

impl PortSlot {
    pub(crate) fn new(type_id: Option<TypeId>, bounds: Cardinality) -> Self {
        Self {
            type_id,
            bounds,
            endpoint: None,
            connected: false,
            claimed: false,
        }
    }

    pub(crate) fn install(&mut self, endpoint: ErasedPort, bounds: Cardinality) {
        self.endpoint = Some(endpoint);
        self.bounds = bounds;
        self.connected = true;
    }

    fn take<T: Send + 'static, P: Any + Send>(
        &mut self,
        port: PortId,
        requested: Cardinality,
        unconnected: impl FnOnce(Cardinality) -> P,
    ) -> Result<P, PortBindingError> {
        if self.claimed {
            return Err(PortBindingError::AlreadyClaimed(port));
        }
        let requested_type = TypeId::of::<T>();
        if let Some(expected) = self.type_id
            && expected != requested_type
        {
            return Err(PortBindingError::TypeMismatch {
                port,
                expected,
                requested: requested_type,
            });
        }
        let compatible = self.bounds.intersection(requested);
        if compatible.is_none() || (self.connected && compatible != Some(self.bounds)) {
            return Err(PortBindingError::CardinalityMismatch {
                port,
                effective: self.bounds,
                requested,
            });
        }
        let bounds = compatible.expect("checked compatible bounds");
        if let Some(endpoint) = &self.endpoint
            && !endpoint.is::<P>()
        {
            return Err(PortBindingError::TypeMismatch {
                port,
                expected: self.type_id.unwrap_or(requested_type),
                requested: requested_type,
            });
        }
        let result = match self.endpoint.take() {
            Some(endpoint) => *endpoint
                .downcast::<P>()
                .unwrap_or_else(|_| unreachable!("checked endpoint type")),
            None => unconnected(bounds),
        };
        self.claimed = true;
        self.type_id = Some(requested_type);
        self.bounds = bounds;
        Ok(result)
    }
}

impl core::fmt::Debug for PortSlot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PortSlot")
            .field("type_id", &self.type_id)
            .field("bounds", &self.bounds)
            .field("connected", &self.connected)
            .field("claimed", &self.claimed)
            .finish()
    }
}

#[derive(Debug, Default)]
pub(crate) struct RuntimePorts {
    pub(crate) inputs: BTreeMap<InputPortId, PortSlot>,
    pub(crate) outputs: BTreeMap<OutputPortId, PortSlot>,
    pub(crate) input_senders: BTreeMap<InputPortId, PortSlot>,
    pub(crate) output_receivers: BTreeMap<OutputPortId, PortSlot>,
}

impl RuntimePorts {
    pub(crate) fn take_input<T: Send + 'static>(
        &mut self,
        id: InputPortId,
        bounds: Cardinality,
    ) -> Result<Inputs<T>, PortBindingError> {
        self.inputs
            .get_mut(&id)
            .ok_or(PortBindingError::UnknownPort(id.into()))?
            .take::<T, _>(id.into(), bounds, Inputs::unconnected)
    }

    pub(crate) fn take_output<T: Send + 'static>(
        &mut self,
        id: OutputPortId,
        bounds: Cardinality,
    ) -> Result<Outputs<T>, PortBindingError> {
        self.outputs
            .get_mut(&id)
            .ok_or(PortBindingError::UnknownPort(id.into()))?
            .take::<T, _>(id.into(), bounds, Outputs::unconnected)
    }

    pub(crate) fn take_input_sender<T: Send + 'static>(
        &mut self,
        id: InputPortId,
        bounds: Cardinality,
    ) -> Result<Outputs<T>, PortBindingError> {
        self.input_senders
            .get_mut(&id)
            .ok_or(PortBindingError::NotExported(id.into()))?
            .take::<T, _>(id.into(), bounds, Outputs::unconnected)
    }

    pub(crate) fn take_output_receiver<T: Send + 'static>(
        &mut self,
        id: OutputPortId,
        bounds: Cardinality,
    ) -> Result<Inputs<T>, PortBindingError> {
        self.output_receivers
            .get_mut(&id)
            .ok_or(PortBindingError::NotExported(id.into()))?
            .take::<T, _>(id.into(), bounds, Inputs::unconnected)
    }
}

/// A process factory's scoped access to its block's runtime endpoints.
///
/// Getters move endpoints exactly once and validate IDs, types, and effective
/// bounds. Returned ports use default const parameters while retaining their
/// actual runtime limits, so existing generic block functions can accept them.
/// Raw access remains guarded by those effective limits.
pub struct BlockPorts<'a> {
    pub(crate) ports: &'a mut RuntimePorts,
    pub(crate) inputs: &'a BTreeSet<InputPortId>,
    pub(crate) outputs: &'a BTreeSet<OutputPortId>,
}

impl BlockPorts<'_> {
    /// Claims an input belonging to this block.
    ///
    /// # Errors
    /// Returns a binding error for foreign, already-claimed, or incompatible ports.
    pub fn take_input<T: Send + 'static, const MAX: isize, const MIN: isize>(
        &mut self,
        port: &crate::model::Inputs<T, MAX, MIN>,
    ) -> Result<Inputs<T>, PortBindingError> {
        if !self.inputs.contains(&port.id()) {
            return Err(PortBindingError::ForeignPort(port.id().into()));
        }
        self.ports.take_input(
            port.id(),
            crate::model::Inputs::<T, MAX, MIN>::message_cardinality(),
        )
    }

    /// Claims an output belonging to this block; clone the returned handle to share it.
    ///
    /// # Errors
    /// Returns a binding error for foreign, already-claimed, or incompatible ports.
    pub fn take_output<T: Send + 'static, const MAX: isize, const MIN: isize>(
        &mut self,
        port: &crate::model::Outputs<T, MAX, MIN>,
    ) -> Result<Outputs<T>, PortBindingError> {
        if !self.outputs.contains(&port.id()) {
            return Err(PortBindingError::ForeignPort(port.id().into()));
        }
        self.ports.take_output(
            port.id(),
            crate::model::Outputs::<T, MAX, MIN>::message_cardinality(),
        )
    }
}
