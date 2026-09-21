// This is free and unencumbered software released into the public domain.

use super::{
    BlockDefinition, InputPortId, OutputPortId, PortId, PortIdMap, PortIdSet, SystemBuilder,
    SystemValidationError,
};
use crate::Cardinality;
use alloc::{
    borrow::Cow,
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
    /// Message-count constraints retained from registration, exports, and connections.
    ///
    /// Multiple declarations on a port are intersected with block metadata.
    /// Missing declarations impose no constraint. Entries must refer to declared ports.
    pub cardinalities: BTreeMap<PortId, Vec<Cardinality>>,
    /// Checked Tokio constructors captured from typed connections and exports.
    ///
    /// Raw metadata edits can use `SystemBuilder::register_message_type` or
    /// insert `ChannelFactory::of::<T>()`; preparation verifies map keys.
    #[cfg(feature = "tokio")]
    pub channel_factories: BTreeMap<TypeId, crate::tokio::ChannelFactory>,
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
    /// Cardinality declarations are intersected per port. Connected inputs must
    /// overlap the sum of their producers' ranges, not each producer separately.
    /// This checks structural compatibility, not whether processes will fulfill
    /// their declared minimums. Runtime receivers check minimums at termination.
    ///
    /// # Errors
    ///
    /// Returns [`SystemValidationError`] for invalid IDs, duplicate block port
    /// ownership, unregistered endpoints or exports, output fan-out, or
    /// conflicting message types. No runtime or channels are created.
    /// Also rejects disjoint cardinalities and unrepresentable aggregate bounds.
    pub fn validate(&self) -> Result<(), SystemValidationError> {
        self.validated_ports().map(|_| ())
    }

    /// Validates this definition and prepares its port wiring for Tokio.
    ///
    /// Requires the `tokio` feature but not an active runtime. Storage is
    /// proportional to the number of declared ports, not the range of their IDs.
    /// Empty systems and definitions with only inputs or only outputs are supported.
    /// Each block must have been registered with `register_executable`. Factories
    /// bind scoped runtime endpoints and construct owned, unpolled futures. No
    /// process starts until `System::execute()` is awaited in a Tokio runtime.
    /// Preparation is repeatable; each call creates fresh channels and futures.
    /// Unconnected ports remain placeholders.
    /// Prepared connections enforce the intersection of their endpoint bounds;
    /// unconnected placeholders retain their declared bounds as well.
    ///
    /// # Errors
    ///
    /// Returns errors for malformed graphs, missing or incorrect channel/process
    /// factories, internally connected exports, and unsuccessful port bindings.
    /// Typed connections/exports retain constructors for `Send + 'static` payloads;
    /// raw type metadata requires explicit constructor registration (`Message`
    /// has a built-in fallback). Exports are boundary-only: inputs expose external
    /// senders and outputs external receivers, obtained from the prepared system.
    /// All validation and binding completes before any future is polled.
    /// On failure, already-created futures and endpoints are dropped.
    ///
    /// Fan-in uses one capacity-one queue per producer and fair input-side polling.
    /// Disconnect markers terminate only their source. Input cardinality constrains
    /// the aggregate; producer minimums reserve slots within a finite shared budget.
    ///
    /// # Panics
    ///
    /// User metadata or factory callbacks may panic. Preparation does not catch them.
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
        self.blocks.push(BlockHandle {
            definition: Rc::clone(block) as _,
            #[cfg(feature = "tokio")]
            executable: None,
        });
    }

    #[cfg(feature = "tokio")]
    pub(crate) fn push_executable<T: crate::tokio::ExecutableBlock + 'static>(
        &mut self,
        block: &Rc<T>,
    ) {
        self.blocks.push(BlockHandle {
            definition: Rc::clone(block) as _,
            executable: Some(Rc::clone(block) as _),
        });
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
            let inputs = block.inputs();
            let outputs = block.outputs();
            for &id in &inputs {
                ports.declare(id.into())?;
                if !block_ports.insert(PortId::Input(id)) {
                    return Err(SystemValidationError::DuplicatePort(id.into()));
                }
                if let Some(type_id) = block.definition.input_type(id) {
                    ports.constrain(id.into(), type_id)?;
                }
                if let Some(cardinality) = block.definition.input_cardinality(id) {
                    ports.constrain_cardinality(id.into(), cardinality)?;
                }
            }
            for &id in &outputs {
                ports.declare(id.into())?;
                if !block_ports.insert(PortId::Output(id)) {
                    return Err(SystemValidationError::DuplicatePort(id.into()));
                }
                if let Some(type_id) = block.definition.output_type(id) {
                    ports.constrain(id.into(), type_id)?;
                }
                if let Some(cardinality) = block.definition.output_cardinality(id) {
                    ports.constrain_cardinality(id.into(), cardinality)?;
                }
            }
            #[cfg(feature = "tokio")]
            ports.blocks.push(ValidatedBlockPorts {
                inputs: inputs.into_iter().collect(),
                outputs: outputs.into_iter().collect(),
            });
        }

        for (&id, &type_id) in self.inputs.iter() {
            ports.constrain(id.into(), type_id)?;
        }
        for (&id, &type_id) in self.outputs.iter() {
            ports.constrain(id.into(), type_id)?;
        }

        for (&port, constraints) in &self.cardinalities {
            ports.constrain_cardinality(port, Cardinality::UNLIMITED)?;
            for &constraint in constraints {
                ports.constrain_cardinality(port, constraint)?;
            }
        }
        let mut connected_outputs = BTreeSet::new();
        let mut producer_ranges = BTreeMap::<InputPortId, Vec<Cardinality>>::new();
        for (&(output, input), &type_id) in &self.connections {
            ports.constrain(output.into(), type_id)?;
            ports.constrain(input.into(), type_id)?;
            if !connected_outputs.insert(output) {
                return Err(SystemValidationError::AlreadyConnectedOutput(output));
            }
            producer_ranges
                .entry(input)
                .or_default()
                .push(ports.cardinality(output.into()));
        }
        for (input, ranges) in producer_ranges {
            let minimum = ranges
                .iter()
                .try_fold(0usize, |sum, range| sum.checked_add(range.min()))
                .ok_or(SystemValidationError::CardinalityOverflow(input))?;
            // Inspect unbounded producers before summing finite maxima so the
            // result does not depend on producer-ID order near usize::MAX.
            let maximum = if ranges.iter().any(|range| range.max().is_none()) {
                None
            } else {
                Some(
                    ranges
                        .iter()
                        .try_fold(0usize, |sum, range| sum.checked_add(range.max()?))
                        .ok_or(SystemValidationError::CardinalityOverflow(input))?,
                )
            };
            let producers =
                Cardinality::new(minimum, maximum).expect("sums of valid ranges are ordered");
            let consumer = ports.cardinality(input.into());
            let effective = producers.intersection(consumer).ok_or(
                SystemValidationError::IncompatibleCardinality {
                    port: input.into(),
                    first: producers,
                    second: consumer,
                },
            )?;
            ports.connection_cardinalities.insert(input, effective);
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
    pub(crate) cardinalities: BTreeMap<PortId, Cardinality>,
    pub(crate) connection_cardinalities: BTreeMap<InputPortId, Cardinality>,
    #[cfg(feature = "tokio")]
    pub(crate) blocks: Vec<ValidatedBlockPorts>,
}

#[cfg(feature = "tokio")]
pub(crate) struct ValidatedBlockPorts {
    pub(crate) inputs: BTreeSet<InputPortId>,
    pub(crate) outputs: BTreeSet<OutputPortId>,
}

impl ValidatedPorts {
    pub(crate) fn cardinality(&self, port: PortId) -> Cardinality {
        self.cardinalities.get(&port).copied().unwrap_or_default()
    }

    fn constrain_cardinality(
        &mut self,
        port: PortId,
        constraint: Cardinality,
    ) -> Result<(), SystemValidationError> {
        Self::check_id(port)?;
        let declared = match port {
            PortId::Input(id) => self.inputs.contains_key(&id),
            PortId::Output(id) => self.outputs.contains_key(&id),
        };
        if !declared {
            return Err(SystemValidationError::UnregisteredPort(port));
        }
        let first = self.cardinality(port);
        let effective = first.intersection(constraint).ok_or(
            SystemValidationError::IncompatibleCardinality {
                port,
                first,
                second: constraint,
            },
        )?;
        self.cardinalities.insert(port, effective);
        Ok(())
    }

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
            .field("cardinalities", &self.cardinalities)
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
pub struct BlockHandle {
    definition: Rc<dyn BlockDefinition>,
    #[cfg(feature = "tokio")]
    pub(crate) executable: Option<Rc<dyn crate::tokio::ExecutableBlock>>,
}

impl BlockHandle {
    /// Returns the block's name from its definition.
    pub fn name(&self) -> Cow<'_, str> {
        self.definition.name()
    }

    pub fn inputs(&self) -> Vec<InputPortId> {
        self.definition.inputs()
    }

    pub fn outputs(&self) -> Vec<OutputPortId> {
        self.definition.outputs()
    }

    pub fn inputs_range(&self) -> Option<RangeInclusive<isize>> {
        let inputs = self.definition.inputs();
        let Some(&min) = inputs.iter().min() else {
            return None;
        };
        let Some(&max) = inputs.iter().max() else {
            unreachable!()
        };
        Some(min.into()..=max.into())
    }

    pub fn outputs_range(&self) -> Option<RangeInclusive<isize>> {
        let outputs = self.definition.outputs();
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
        let inputs = PortIdSet::from(&self.definition.inputs());
        let outputs = PortIdSet::from(&self.definition.outputs());
        f.debug_struct(&self.definition.name())
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
        // Defensive validation still rejects malformed crate-internal values.
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
