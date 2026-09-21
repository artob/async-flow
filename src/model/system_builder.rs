// This is free and unencumbered software released into the public domain.

use super::{
    BlockDefinition, InputPortId, Inputs, OutputPortId, Outputs, PortExport, PortId, PortIdSet,
    PortRegistration, SystemDefinition,
};
use alloc::rc::Rc;
use core::{any::TypeId, fmt::Debug};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum SystemBuildError {
    #[error("unregistered input port ID: {0}")]
    UnregisteredInput(InputPortId),

    #[error("unregistered output port ID: {0}")]
    UnregisteredOutput(OutputPortId),

    #[error("already connected output port ID: {0}")]
    AlreadyConnectedOutput(OutputPortId),
}

/// A builder for system definitions.
///
/// # Examples
///
/// ```
/// use async_flow::model::SystemBuilder;
///
/// let mut builder = SystemBuilder::new();
/// //let block = builder.register(MyBlock::new());
/// let system = builder.build();
/// ```
#[derive(Clone, Default)]
pub struct SystemBuilder {
    system: SystemDefinition,
    registered_inputs: PortIdSet<InputPortId>,
    registered_outputs: PortIdSet<OutputPortId>,
    connected_outputs: PortIdSet<OutputPortId>,
}

impl SystemBuilder {
    /// Creates a new system builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a block definition and its ports with the system under construction.
    ///
    /// Registration records structural metadata; it does not start a block process.
    pub fn register<T: BlockDefinition + 'static>(&mut self, block: T) -> Rc<T> {
        let block: Rc<T> = Rc::new(block);
        self.system.push_block(&block);

        for input in block.inputs() {
            self.registered_inputs.insert(input);
        }
        for output in block.outputs() {
            self.registered_outputs.insert(output);
        }

        block
    }

    /// Registers a block definition with its reusable Tokio process factory.
    ///
    /// Requires `tokio`. Registration does not create or start a process;
    /// preparation creates an unpolled future, and execution starts it.
    #[cfg(feature = "tokio")]
    pub fn register_executable<T: crate::tokio::ExecutableBlock + 'static>(
        &mut self,
        block: T,
    ) -> Rc<T> {
        let block = Rc::new(block);
        self.system.push_executable(&block);
        for input in block.inputs() {
            self.registered_inputs.insert(input);
        }
        for output in block.outputs() {
            self.registered_outputs.insert(output);
        }
        block
    }

    /// Registers a Tokio channel constructor for raw `TypeId` connections or exports.
    ///
    /// Typed `connect` and descriptor exports register their types automatically.
    /// This operation requires `tokio` but no active runtime.
    #[cfg(feature = "tokio")]
    pub fn register_message_type<T: Send + 'static>(&mut self) {
        self.system
            .channel_factories
            .insert(TypeId::of::<T>(), crate::tokio::ChannelFactory::of::<T>());
    }

    /// Registers an input or output port with the system under construction.
    /// Descriptor references preserve cardinality; raw IDs impose no constraint.
    pub fn register_port(&mut self, input: impl Into<PortRegistration<PortId>>) {
        let PortRegistration { id, cardinality } = input.into();
        match id {
            PortId::Input(id) => self.register_input(PortRegistration { id, cardinality }),
            PortId::Output(id) => self.register_output(PortRegistration { id, cardinality }),
        }
    }

    /// Registers an input port with the system under construction.
    ///
    /// The registration is retained by the definition, including for ports
    /// that are neither exported nor connected. Identical registrations are a no-op;
    /// additional cardinality declarations are retained for intersection at validation.
    pub fn register_input(&mut self, input: impl Into<PortRegistration<InputPortId>>) {
        let PortRegistration {
            id: input,
            cardinality,
        } = input.into();
        self.registered_inputs.insert(input);
        self.system.registered_inputs.insert(input);
        self.record_cardinality(input.into(), cardinality);
    }

    /// Registers an output port with the system under construction.
    ///
    /// The registration is retained by the definition, including for ports
    /// that are neither exported nor connected. Identical registrations are a no-op;
    /// additional cardinality declarations are retained for intersection at validation.
    pub fn register_output(&mut self, output: impl Into<PortRegistration<OutputPortId>>) {
        let PortRegistration {
            id: output,
            cardinality,
        } = output.into();
        self.registered_outputs.insert(output);
        self.system.registered_outputs.insert(output);
        self.record_cardinality(output.into(), cardinality);
    }

    /// Exports an input or output port registered with the system under
    /// construction.
    pub fn export(
        &mut self,
        input: impl Into<PortExport<PortId>>,
    ) -> Result<PortId, SystemBuildError> {
        self.export_port(input)
    }

    /// Exports an input or output port registered with the system under
    /// construction.
    pub fn export_port(
        &mut self,
        input: impl Into<PortExport<PortId>>,
    ) -> Result<PortId, SystemBuildError> {
        let PortExport {
            id: input,
            type_id,
            cardinality,
            #[cfg(feature = "tokio")]
            channel_factory,
        } = input.into();
        match input {
            PortId::Input(id) => self
                .export_input(PortExport {
                    id,
                    type_id,
                    cardinality,
                    #[cfg(feature = "tokio")]
                    channel_factory,
                })
                .map(|_| ()),
            PortId::Output(id) => self
                .export_output(PortExport {
                    id,
                    type_id,
                    cardinality,
                    #[cfg(feature = "tokio")]
                    channel_factory,
                })
                .map(|_| ()),
        }?;
        Ok(input)
    }

    /// Exports an input port registered with the system under construction.
    pub fn export_input(
        &mut self,
        input: impl Into<PortExport<InputPortId>>,
    ) -> Result<InputPortId, SystemBuildError> {
        let PortExport {
            id: input,
            type_id,
            cardinality,
            #[cfg(feature = "tokio")]
            channel_factory,
        } = input.into();
        if !self.registered_inputs.contains(input) {
            return Err(SystemBuildError::UnregisteredInput(input));
        }
        self.system.inputs.insert(input, type_id);
        #[cfg(feature = "tokio")]
        if let Some(factory) = channel_factory {
            self.system.channel_factories.insert(type_id, factory);
        }
        self.record_cardinality(input.into(), cardinality);
        Ok(input)
    }

    /// Exports an output port registered with the system under construction.
    pub fn export_output(
        &mut self,
        output: impl Into<PortExport<OutputPortId>>,
    ) -> Result<OutputPortId, SystemBuildError> {
        let PortExport {
            id: output,
            type_id,
            cardinality,
            #[cfg(feature = "tokio")]
            channel_factory,
        } = output.into();
        if !self.registered_outputs.contains(output) {
            return Err(SystemBuildError::UnregisteredOutput(output));
        }
        self.system.outputs.insert(output, type_id);
        #[cfg(feature = "tokio")]
        if let Some(factory) = channel_factory {
            self.system.channel_factories.insert(type_id, factory);
        }
        self.record_cardinality(output.into(), cardinality);
        Ok(output)
    }

    /// Connects an output port to an input port of the same type.
    ///
    /// Retains each port's cardinality, including nondefault bounds. Compatibility
    /// is checked by definition validation after the full set of producers is known.
    /// Returns `true` for a new connection; reconnecting an output is an error.
    pub fn connect<
        T: Send + 'static,
        const OUT_MAX: isize,
        const OUT_MIN: isize,
        const IN_MAX: isize,
        const IN_MIN: isize,
    >(
        &mut self,
        output: &Outputs<T, OUT_MAX, OUT_MIN>,
        input: &Inputs<T, IN_MAX, IN_MIN>,
    ) -> Result<bool, SystemBuildError> {
        let inserted = self.connect_ports(output.id(), input.id(), TypeId::of::<T>())?;
        #[cfg(feature = "tokio")]
        self.register_message_type::<T>();
        self.record_cardinality(
            output.id().into(),
            Some(Outputs::<T, OUT_MAX, OUT_MIN>::message_cardinality()),
        );
        self.record_cardinality(
            input.id().into(),
            Some(Inputs::<T, IN_MAX, IN_MIN>::message_cardinality()),
        );
        Ok(inserted)
    }

    /// Connects an output port ID to an input port ID.
    /// This isn't public because it doesn't enforce type safety.
    ///
    /// Returns a boolean indicating whether the connection was newly
    /// inserted or already existed.
    pub(crate) fn connect_ports(
        &mut self,
        output: impl Into<OutputPortId>,
        input: impl Into<InputPortId>,
        type_id: TypeId,
    ) -> Result<bool, SystemBuildError> {
        let output = output.into();
        let input = input.into();
        if !self.registered_inputs.contains(input) {
            return Err(SystemBuildError::UnregisteredInput(input));
        }
        if !self.registered_outputs.contains(output) {
            return Err(SystemBuildError::UnregisteredOutput(output));
        }
        if self.connected_outputs.contains(output) {
            return Err(SystemBuildError::AlreadyConnectedOutput(output));
        }
        let result = self
            .system
            .connections
            .insert((output, input), type_id)
            .is_none();
        if result {
            // Output ports can only be connected once:
            self.connected_outputs.insert(output);
        }
        Ok(result)
    }

    /// Builds the system under construction.
    ///
    /// This retains explicit port registrations but does not perform full graph
    /// validation. Call [`SystemDefinition::validate`] to check block ownership,
    /// endpoint membership, and type consistency. Tokio preparation validates
    /// the definition again before allocating channels.
    pub fn build(self) -> SystemDefinition {
        self.system
    }

    fn record_cardinality(&mut self, port: PortId, cardinality: Option<crate::Cardinality>) {
        if let Some(cardinality) = cardinality {
            let constraints = self.system.cardinalities.entry(port).or_default();
            if !constraints.contains(&cardinality) {
                constraints.push(cardinality);
            }
        }
    }
}

impl Debug for SystemBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SystemBuilder")
            .field("registered_inputs", &self.registered_inputs)
            .field("registered_outputs", &self.registered_outputs)
            .field("connected_outputs", &self.connected_outputs)
            .field("system", &self.system)
            .finish()
    }
}
