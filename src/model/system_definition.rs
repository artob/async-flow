// This is free and unencumbered software released into the public domain.

use super::{
    BlockDefinition, InputPortId, OutputPortId, PortId, PortIdMap, PortIdSet, SystemBuilder,
    SystemValidationError,
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    vec::Vec,
};
use core::{any::TypeId, fmt::Debug, ops::RangeInclusive};

/// The structural definition of a system of connected blocks.
///
/// Its graph describes blocks, their ports, and the connections between ports.
/// Executing a block requires a process, supplied separately by a runtime backend.
///
/// Ports are declared by blocks or by explicit registration. Exporting a port
/// does not register it. Because graph fields are editable, call
/// [`validate`](Self::validate) after edits; runtime preparation always validates
/// the current definition, rather than trusting earlier builder checks.
#[derive(Clone, Default)]
pub struct SystemDefinition {
    /// Exported inputs and their message types.
    pub inputs: PortIdMap<InputPortId, TypeId>,
    /// Exported outputs and their message types.
    pub outputs: PortIdMap<OutputPortId, TypeId>,
    /// Block definitions declaring the system's ports.
    pub blocks: Vec<BlockHandle>,
    /// Output-to-input connections (graph edges) and their message types.
    pub connections: BTreeMap<(OutputPortId, InputPortId), TypeId>,
    /// Explicitly registered inputs, including standalone, unconnected ports.
    ///
    /// Block ports are discovered from `blocks`; they need not be listed here.
    pub registered_inputs: PortIdSet<InputPortId>,
    /// Explicitly registered outputs, including standalone, unconnected ports.
    ///
    /// Block ports are discovered from `blocks`; they need not be listed here.
    pub registered_outputs: PortIdSet<OutputPortId>,
}

impl SystemDefinition {
    /// Returns a system builder.
    pub fn build() -> SystemBuilder {
        SystemBuilder::new()
    }

    /// Checks port IDs, block ownership, exports, connections, and message types.
    ///
    /// Empty systems, unconnected ports, cycles, and inputs with multiple
    /// producers are structurally valid. An output may have only one connection.
    /// A backend may impose additional preparation restrictions.
    ///
    /// Block type metadata is checked first, followed by exported types and
    /// connection types. Ports without declared types infer their types from
    /// exports or connections; validation cannot verify undeclared block types.
    /// Each block's port lists and type metadata are read once per validation.
    ///
    /// # Errors
    ///
    /// Returns [`SystemValidationError`] for invalid IDs, duplicate block port
    /// ownership, unregistered endpoints or exports, output fan-out, or
    /// conflicting message types. No runtime or channels are created.
    pub fn validate(&self) -> Result<(), SystemValidationError> {
        self.validated_ports().map(|_| ())
    }

    /// Validates this definition and prepares its port wiring for Tokio.
    ///
    /// Requires the `tokio` feature but not an active runtime. Storage is
    /// proportional to the number of declared ports, not the range of their IDs.
    /// Empty systems and definitions with only inputs or only outputs are supported.
    /// Starting processes from block definitions is not implemented yet;
    /// unconnected ports remain placeholders.
    ///
    /// # Errors
    ///
    /// Returns a validation error for a malformed graph. The current Tokio
    /// preparation also rejects fan-in and connections whose declared message
    /// type is not the concrete [`crate::Message`] alias. This is a limitation of
    /// system preparation; generic runtime ports can carry other message types.
    /// Validation and backend checks finish before any channels are allocated.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_flow::{Message, model::{Inputs, Outputs, SystemBuilder}};
    ///
    /// let input = Inputs::<Message>::default();
    /// let output = Outputs::<Message>::default();
    /// let mut builder = SystemBuilder::new();
    /// builder.register_input(&input);
    /// builder.register_output(&output);
    /// builder.connect(&output, &input)?;
    /// let system = builder.build().prepare()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(feature = "tokio")]
    pub fn prepare(&self) -> Result<crate::tokio::System, crate::tokio::SystemPrepareError> {
        self.try_into()
    }

    pub(crate) fn push_block<T: BlockDefinition + 'static>(&mut self, block: &Rc<T>) {
        self.blocks.push(BlockHandle(Rc::clone(block) as _));
    }

    /// Returns the numerically smallest declared input ID.
    pub fn inputs_min(&self) -> Option<InputPortId> {
        self.inputs_range().map(|r| InputPortId(*r.start()))
    }

    /// Returns the numerically largest declared input ID.
    pub fn inputs_max(&self) -> Option<InputPortId> {
        self.inputs_range().map(|r| InputPortId(*r.end()))
    }

    /// Returns the bounds of registered and block input IDs, or `None` if empty.
    ///
    /// IDs within these bounds need not be declared; this is not a port count.
    pub fn inputs_range(&self) -> Option<RangeInclusive<isize>> {
        port_range(
            self.registered_inputs
                .iter()
                .copied()
                .chain(self.blocks.iter().flat_map(BlockHandle::inputs))
                .map(isize::from),
        )
    }

    /// Returns the numerically smallest declared output ID.
    pub fn outputs_min(&self) -> Option<OutputPortId> {
        self.outputs_range().map(|r| OutputPortId(*r.start()))
    }

    /// Returns the numerically largest declared output ID.
    pub fn outputs_max(&self) -> Option<OutputPortId> {
        self.outputs_range().map(|r| OutputPortId(*r.end()))
    }

    /// Returns the bounds of registered and block output IDs, or `None` if empty.
    ///
    /// IDs within these bounds need not be declared; this is not a port count.
    pub fn outputs_range(&self) -> Option<RangeInclusive<isize>> {
        port_range(
            self.registered_outputs
                .iter()
                .copied()
                .chain(self.blocks.iter().flat_map(BlockHandle::outputs))
                .map(isize::from),
        )
    }

    pub(crate) fn validated_ports(&self) -> Result<ValidatedPorts, SystemValidationError> {
        let mut ports = ValidatedPorts::default();
        for &id in self.registered_inputs.iter() {
            ports.declare(id.into())?;
        }
        for &id in self.registered_outputs.iter() {
            ports.declare(id.into())?;
        }

        let mut block_ports = BTreeSet::new();
        for block in &self.blocks {
            for id in block.inputs() {
                ports.declare(id.into())?;
                if !block_ports.insert(PortId::Input(id)) {
                    return Err(SystemValidationError::DuplicatePort(id.into()));
                }
                if let Some(type_id) = block.0.input_type(id) {
                    ports.constrain(id.into(), type_id)?;
                }
            }
            for id in block.outputs() {
                ports.declare(id.into())?;
                if !block_ports.insert(PortId::Output(id)) {
                    return Err(SystemValidationError::DuplicatePort(id.into()));
                }
                if let Some(type_id) = block.0.output_type(id) {
                    ports.constrain(id.into(), type_id)?;
                }
            }
        }

        for (&id, &type_id) in self.inputs.iter() {
            ports.constrain(id.into(), type_id)?;
        }
        for (&id, &type_id) in self.outputs.iter() {
            ports.constrain(id.into(), type_id)?;
        }

        let mut connected_outputs = BTreeSet::new();
        for (&(output, input), &type_id) in &self.connections {
            ports.constrain(output.into(), type_id)?;
            ports.constrain(input.into(), type_id)?;
            if !connected_outputs.insert(output) {
                return Err(SystemValidationError::AlreadyConnectedOutput(output));
            }
        }
        Ok(ports)
    }
}

fn port_range(mut ids: impl Iterator<Item = isize>) -> Option<RangeInclusive<isize>> {
    let first = ids.next()?;
    let (min, max) = ids.fold((first, first), |(min, max), id| (min.min(id), max.max(id)));
    Some(min..=max)
}

#[derive(Default)]
pub(crate) struct ValidatedPorts {
    pub(crate) inputs: BTreeMap<InputPortId, Option<TypeId>>,
    pub(crate) outputs: BTreeMap<OutputPortId, Option<TypeId>>,
}

impl ValidatedPorts {
    fn check_id(id: PortId) -> Result<(), SystemValidationError> {
        match id {
            PortId::Input(id) if isize::from(id) < 0 => Ok(()),
            PortId::Output(id) if isize::from(id) > 0 => Ok(()),
            _ => Err(SystemValidationError::InvalidPortId(id)),
        }
    }

    fn declare(&mut self, id: PortId) -> Result<(), SystemValidationError> {
        Self::check_id(id)?;
        match id {
            PortId::Input(id) => {
                self.inputs.entry(id).or_insert(None);
            },
            PortId::Output(id) => {
                self.outputs.entry(id).or_insert(None);
            },
        }
        Ok(())
    }

    fn constrain(&mut self, port: PortId, actual: TypeId) -> Result<(), SystemValidationError> {
        Self::check_id(port)?;
        let expected = match port {
            PortId::Input(id) => self.inputs.get_mut(&id),
            PortId::Output(id) => self.outputs.get_mut(&id),
        }
        .ok_or(SystemValidationError::UnregisteredPort(port))?;
        match *expected {
            Some(expected) if expected != actual => Err(SystemValidationError::TypeMismatch {
                port,
                expected,
                actual,
            }),
            _ => {
                *expected = Some(actual);
                Ok(())
            },
        }
    }
}

impl Debug for SystemDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SystemDefinition")
            .field("registered_inputs", &self.registered_inputs)
            .field("registered_outputs", &self.registered_outputs)
            .field(
                "inputs",
                &self
                    .inputs
                    .iter()
                    .map(|(id, typ)| (id.0, typ))
                    .collect::<Vec<_>>(),
            )
            .field(
                "outputs",
                &self
                    .outputs
                    .iter()
                    .map(|(id, typ)| (id.0, typ))
                    .collect::<Vec<_>>(),
            )
            .field("blocks", &self.blocks)
            .field(
                "connections",
                &self
                    .connections
                    .iter()
                    .map(|((from, to), typ)| ((from.0, to.0), typ))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// A shared handle to a block definition in a system's structural graph.
#[derive(Clone)]
pub struct BlockHandle(Rc<dyn BlockDefinition>);

impl BlockHandle {
    pub fn inputs(&self) -> Vec<InputPortId> {
        self.0.inputs()
    }

    pub fn outputs(&self) -> Vec<OutputPortId> {
        self.0.outputs()
    }

    pub fn inputs_range(&self) -> Option<RangeInclusive<isize>> {
        let inputs = self.0.inputs();
        let Some(&min) = inputs.iter().min() else {
            return None;
        };
        let Some(&max) = inputs.iter().max() else {
            unreachable!()
        };
        Some(min.into()..=max.into())
    }

    pub fn outputs_range(&self) -> Option<RangeInclusive<isize>> {
        let outputs = self.0.outputs();
        let Some(&min) = outputs.iter().min() else {
            return None;
        };
        let Some(&max) = outputs.iter().max() else {
            unreachable!()
        };
        Some(min.into()..=max.into())
    }
}

impl Debug for BlockHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inputs = PortIdSet::from(&self.0.inputs());
        let outputs = PortIdSet::from(&self.0.outputs());
        f.debug_struct(&self.0.name())
            .field("inputs", &inputs)
            .field("outputs", &outputs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ids_are_rejected_in_declarations_exports_and_connections() {
        // These values can currently also be constructed through deserialization.
        for port in [
            PortId::Input(InputPortId(0)),
            PortId::Input(InputPortId(1)),
            PortId::Output(OutputPortId(0)),
            PortId::Output(OutputPortId(-1)),
        ] {
            for location in 0..3 {
                let mut graph = SystemDefinition::default();
                match (port, location) {
                    (PortId::Input(id), 0) => {
                        graph.registered_inputs.insert(id);
                    },
                    (PortId::Output(id), 0) => {
                        graph.registered_outputs.insert(id);
                    },
                    (PortId::Input(id), 1) => {
                        graph.inputs.insert(id, TypeId::of::<u8>());
                    },
                    (PortId::Output(id), 1) => {
                        graph.outputs.insert(id, TypeId::of::<u8>());
                    },
                    (PortId::Input(id), _) => {
                        graph.registered_outputs.insert(OutputPortId(1));
                        graph
                            .connections
                            .insert((OutputPortId(1), id), TypeId::of::<u8>());
                    },
                    (PortId::Output(id), _) => {
                        graph.registered_inputs.insert(InputPortId(-1));
                        graph
                            .connections
                            .insert((id, InputPortId(-1)), TypeId::of::<u8>());
                    },
                }
                assert_eq!(
                    graph.validate(),
                    Err(SystemValidationError::InvalidPortId(port))
                );
            }
        }
    }
}
