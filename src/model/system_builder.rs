// This is free and unencumbered software released into the public domain.

use super::{
    BlockDefinition, InputPortId, Inputs, OutputPortId, Outputs, PortId, PortIdSet,
    SystemDefinition,
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

    /// Registers an instantiated block with the system under construction.
    pub fn register<T: BlockDefinition + 'static>(&mut self, block: T) -> Rc<T> {
        let block: Rc<T> = Rc::new(block);
        self.system.push_block(&block);

        for input in block.inputs() {
            self.register_input(input);
        }
        for output in block.outputs() {
            self.register_output(output);
        }

        block
    }

    /// Registers an input or output port with the system under construction.
    pub fn register_port(&mut self, input: impl Into<PortId>) {
        match input.into() {
            PortId::Input(input) => self.register_input(input),
            PortId::Output(output) => self.register_output(output),
        }
    }

    /// Registers an input port with the system under construction.
    pub fn register_input(&mut self, input: impl Into<InputPortId>) {
        let input = input.into();
        self.registered_inputs.insert(input);
    }

    /// Registers an output port with the system under construction.
    pub fn register_output(&mut self, output: impl Into<OutputPortId>) {
        let output = output.into();
        self.registered_outputs.insert(output);
    }

    /// Exports an input or output port registered with the system under
    /// construction.
    pub fn export(&mut self, input: impl Into<PortId>) -> Result<PortId, SystemBuildError> {
        self.export_port(input)
    }

    /// Exports an input or output port registered with the system under
    /// construction.
    pub fn export_port(&mut self, input: impl Into<PortId>) -> Result<PortId, SystemBuildError> {
        let input = input.into();
        match input.into() {
            PortId::Input(input) => self.export_input(input).map(|_| ()),
            PortId::Output(output) => self.export_output(output).map(|_| ()),
        }?;
        Ok(input)
    }

    /// Exports an input port registered with the system under construction.
    pub fn export_input(
        &mut self,
        input: impl Into<InputPortId>,
    ) -> Result<InputPortId, SystemBuildError> {
        let input = input.into();
        if !self.registered_inputs.contains(input) {
            return Err(SystemBuildError::UnregisteredInput(input));
        }
        self.system.inputs.insert(input);
        Ok(input)
    }

    /// Exports an output port registered with the system under construction.
    pub fn export_output(
        &mut self,
        output: impl Into<OutputPortId>,
    ) -> Result<OutputPortId, SystemBuildError> {
        let output = output.into();
        if !self.registered_outputs.contains(output) {
            return Err(SystemBuildError::UnregisteredOutput(output));
        }
        self.system.outputs.insert(output);
        Ok(output)
    }

    /// Connects an output port to an input port of the same type.
    ///
    /// Returns a boolean indicating whether the connection was newly
    /// inserted or already existed.
    pub fn connect<T: 'static>(
        &mut self,
        output: &Outputs<T>,
        input: &Inputs<T>,
    ) -> Result<bool, SystemBuildError> {
        self.connect_ports(output.id(), input.id(), TypeId::of::<T>())
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
    pub fn build(self) -> SystemDefinition {
        self.system
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
