// This is free and unencumbered software released into the public domain.

use super::{Channel, Inputs, Outputs};
use crate::{error::Result, io::Message, model::SystemDefinition};
use alloc::vec::Vec;
use tokio::task::{AbortHandle, JoinSet};

pub type Subsystem = System;

#[derive(Debug, Default)]
pub struct System {
    pub(crate) inputs: Vec<Inputs<Message>>,
    pub(crate) outputs: Vec<Outputs<Message>>,
    pub(crate) blocks: JoinSet<Result>,
}

impl System {
    // pub fn oneshot<T>() -> Channel<T, ONESHOT> {
    //     Channel::oneshot()
    // }

    // pub fn bounded<T>(buffer: usize) -> Channel<T, UNLIMITED> {
    //     Channel::bounded(buffer)
    // }

    /// Builds and executes a system, blocking until completion.
    pub async fn run<F: FnOnce(&mut Self)>(f: F) -> Result {
        Self::build(f).execute().await
    }

    /// Builds a new system.
    pub fn build<F: FnOnce(&mut Self)>(f: F) -> Self {
        let mut system = Self::new();
        f(&mut system);
        system
    }

    /// Instantiates a new system.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect<T>(&mut self, inputs: Inputs<T>, outputs: Outputs<T>)
    where
        T: Send + 'static,
    {
        self.blocks.spawn(async move {
            let mut inputs = inputs;
            let outputs = outputs;
            while let Some(input) = inputs.recv().await? {
                outputs.send(input).await?;
            }
            Ok(())
        });
    }

    pub fn spawn<F>(&mut self, task: F) -> AbortHandle
    where
        F: Future<Output = Result>,
        F: Send + 'static,
    {
        self.blocks.spawn(task)
    }

    pub async fn execute(self) -> Result {
        self.blocks.join_all().await;
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn read_stdin<T: core::str::FromStr>(&mut self) -> Inputs<T>
    where
        T: Send + 'static,
        <T as core::str::FromStr>::Err: Send,
    {
        let (output, input) = super::Channel::<T>::bounded(1).into_inner(); // TODO
        let block = super::stdin(output);
        self.blocks.spawn(block);
        input
    }

    #[cfg(feature = "std")]
    pub fn write_stdout<T: alloc::string::ToString>(&mut self) -> Outputs<T>
    where
        T: Send + 'static,
    {
        let (output, input) = super::Channel::<T>::bounded(1).into_inner(); // TODO
        let block = super::stdout(input);
        self.blocks.spawn(block);
        output
    }
}

impl From<&SystemDefinition> for System {
    fn from(system_definition: &SystemDefinition) -> Self {
        let mut system = Self::new();

        let input_max = system_definition.inputs_max().unwrap();
        let input_ids = system_definition.inputs_range().unwrap();
        let input_count = input_ids.count();
        system.inputs.resize_with(input_count, Inputs::default);

        let output_min = system_definition.outputs_min().unwrap();
        let output_ids = system_definition.outputs_range().unwrap();
        let output_count = output_ids.count();
        system.outputs.resize_with(output_count, Outputs::default);

        for ((output_id, input_id), _) in &system_definition.connections {
            // TODO: support multiple connections to the same input port
            let channel = Channel::<Message>::bounded(1);
            let output_index = output_id.index() - output_min.index();
            let input_index = input_id.index() - input_max.index();
            system.outputs[output_index] = channel.tx;
            system.inputs[input_index] = channel.rx;
        }

        // TODO: schedule system.blocks
        for _block in &system_definition.blocks {
            //system.blocks.spawn(block);
        }

        system
    }
}
