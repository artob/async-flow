// This is free and unencumbered software released into the public domain.

use super::{Channel, Inputs, ONESHOT, Outputs, UNLIMITED};
use crate::io::Result;
use tokio::task::{AbortHandle, JoinSet};

pub type Subsystem = System;

pub struct System {
    pub(crate) blocks: JoinSet<Result>,
}

impl System {
    pub fn oneshot<T>() -> Channel<T, ONESHOT> {
        Channel::oneshot()
    }

    pub fn bounded<T>(buffer: usize) -> Channel<T, UNLIMITED> {
        Channel::bounded(buffer)
    }

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
        Self {
            blocks: JoinSet::new(),
        }
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
        let (output, input) = super::Channel::bounded(1).into_inner(); // TODO
        let block = super::stdin(output);
        self.blocks.spawn(block);
        input
    }

    #[cfg(feature = "std")]
    pub fn write_stdout<T: alloc::string::ToString>(&mut self) -> Outputs<T>
    where
        T: Send + 'static,
    {
        let (output, input) = super::Channel::bounded(1).into_inner(); // TODO
        let block = super::stdout(input);
        self.blocks.spawn(block);
        output
    }
}
