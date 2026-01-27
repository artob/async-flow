// This is free and unencumbered software released into the public domain.

use super::{BlockDefinition, InputId, Inputs, OutputId, Outputs, SystemDefinition};
use alloc::{collections::BTreeSet, rc::Rc};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum SystemBuildError {
    #[error("unregistered input port ID: {0}")]
    UnregisteredInput(InputId),

    #[error("unregistered output port ID: {0}")]
    UnregisteredOutput(OutputId),

    #[error("already connected output port ID: {0}")]
    AlreadyConnectedOutput(OutputId),
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
#[derive(Clone, Debug, Default)]
pub struct SystemBuilder {
    system: SystemDefinition,
    registered_inputs: BTreeSet<InputId>,
    registered_outputs: BTreeSet<OutputId>,
    connected_outputs: BTreeSet<OutputId>,
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

    /// Registers an input port with the system under construction.
    pub fn register_input(&mut self, input: impl Into<InputId>) {
        let input = input.into();
        self.registered_inputs.insert(input);
    }

    /// Registers an output port with the system under construction.
    pub fn register_output(&mut self, output: impl Into<OutputId>) {
        let output = output.into();
        self.registered_outputs.insert(output);
    }

    /// Exports an input port registered with the system under construction.
    pub fn export_input(&mut self, input: impl Into<InputId>) -> Result<InputId, SystemBuildError> {
        let input = input.into();
        if !self.registered_inputs.contains(&input) {
            return Err(SystemBuildError::UnregisteredInput(input));
        }
        self.system.inputs.insert(input);
        Ok(input)
    }

    /// Exports an output port registered with the system under construction.
    pub fn export_output(
        &mut self,
        output: impl Into<OutputId>,
    ) -> Result<OutputId, SystemBuildError> {
        let output = output.into();
        if !self.registered_outputs.contains(&output) {
            return Err(SystemBuildError::UnregisteredOutput(output));
        }
        self.system.outputs.insert(output);
        Ok(output)
    }

    /// Connects an output port to an input port of the same type.
    ///
    /// Returns a boolean indicating whether the connection was newly
    /// inserted or already existed.
    pub fn connect<T>(
        &mut self,
        output: &Outputs<T>,
        input: &Inputs<T>,
    ) -> Result<bool, SystemBuildError> {
        self.connect_ids(output.id(), input.id())
    }

    /// Connects an output port ID to an input port ID.
    /// This isn't public because it doesn't enforce type safety.
    ///
    /// Returns a boolean indicating whether the connection was newly
    /// inserted or already existed.
    pub(crate) fn connect_ids(
        &mut self,
        output: impl Into<OutputId>,
        input: impl Into<InputId>,
    ) -> Result<bool, SystemBuildError> {
        let output = output.into();
        let input = input.into();
        if !self.registered_inputs.contains(&input) {
            return Err(SystemBuildError::UnregisteredInput(input));
        }
        if !self.registered_outputs.contains(&output) {
            return Err(SystemBuildError::UnregisteredOutput(output));
        }
        if self.connected_outputs.contains(&output) {
            return Err(SystemBuildError::AlreadyConnectedOutput(output));
        }
        let result = self.system.connections.insert((output, input));
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
