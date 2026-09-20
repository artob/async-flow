// This is free and unencumbered software released into the public domain.

use super::{Channel, Inputs, Outputs};
use crate::{error::Result, io::Message, model::SystemDefinition};
use alloc::vec::Vec;
use tokio::task::{AbortHandle, JoinSet};

pub type Subsystem = System;

/// A collection of Tokio tasks connected through ports.
///
/// Tasks are scheduled when spawned, rather than when [`execute`](Self::execute) is
/// called. Spawning requires an active Tokio runtime, which must remain running
/// while the tasks execute. Dropping a system aborts its remaining tasks without
/// waiting for them to finish; await `execute()` to join them.
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

    /// Builds a system and asynchronously waits for its tasks to finish.
    ///
    /// See [`build`](Self::build) for runtime requirements and
    /// [`execute`](Self::execute) for error propagation and cancellation behavior.
    pub async fn run<F: FnOnce(&mut Self)>(f: F) -> Result {
        Self::build(f).execute().await
    }

    /// Builds a new system.
    ///
    /// Tasks spawned by `f` are scheduled immediately on the active Tokio runtime.
    ///
    /// # Panics
    ///
    /// Panics if `f` panics, including if it spawns tasks outside a Tokio runtime.
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

    /// Spawns a block on the active Tokio runtime and returns its abort handle.
    ///
    /// The block is scheduled immediately. [`execute`](Self::execute) observes its
    /// result; an aborted block is reported as [`crate::Error::Join`].
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn spawn<F>(&mut self, task: F) -> AbortHandle
    where
        F: Future<Output = Result>,
        F: Send + 'static,
    {
        self.blocks.spawn(task)
    }

    /// Waits for all blocks to finish, stopping on the first observed failure.
    ///
    /// Returns `Ok(())` when every block succeeds, including for an empty system.
    /// Tasks have already been spawned; their Tokio runtime must remain running
    /// while this future is awaited.
    ///
    /// # Errors
    ///
    /// Returns the first error observed while joining blocks, in completion
    /// order rather than spawn order. Block errors are returned unchanged; task
    /// panics and cancellations are returned as [`crate::Error::Join`].
    ///
    /// On failure, all remaining blocks are aborted and joined before the error
    /// is returned. Additional errors or panics during shutdown are ignored.
    /// Cancellation is cooperative: a task that does not yield can prevent
    /// shutdown from completing. Buffered messages may be discarded on failure.
    ///
    /// # Cancellation
    ///
    /// Dropping this future aborts remaining blocks without waiting for cleanup.
    /// Await it to completion to ensure that all blocks have been joined.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_flow::{Error, SendError, tokio::System};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut system = System::new();
    /// system.spawn(async { Err(SendError::Closed.into()) });
    ///
    /// let result = system.execute().await;
    /// assert!(matches!(result, Err(Error::Send(SendError::Closed))));
    /// # }
    /// ```
    pub async fn execute(mut self) -> Result {
        while let Some(result) = self.blocks.join_next().await {
            let error = match result {
                Ok(Ok(())) => continue,
                Ok(Err(error)) => error,
                Err(error) => error.into(),
            };
            self.blocks.shutdown().await;
            return Err(error);
        }
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
