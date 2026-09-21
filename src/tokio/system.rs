// This is free and unencumbered software released into the public domain.

use super::{Channel, Inputs, Outputs, SystemPrepareError};
use crate::{
    error::Result,
    io::Message,
    model::{InputPortId, OutputPortId, SystemDefinition},
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::any::TypeId;
use tokio::task::{AbortHandle, JoinSet};

/// A system used within another system; currently an alias for [`System`].
pub type Subsystem = System;

/// A Tokio execution context for a system of connected blocks.
///
/// A system comprises blocks that exchange messages through ports. A block's
/// execution is a process, represented here by a Tokio task. Runtime threads
/// drive those tasks.
///
/// Tasks are scheduled when spawned, rather than when [`execute`](Self::execute) is
/// called. Spawning requires an active Tokio runtime, which must remain running
/// while the tasks execute. Dropping a system aborts its remaining tasks without
/// waiting for them to finish; await `execute()` to join them.
#[derive(Debug, Default)]
pub struct System {
    pub(crate) inputs: Vec<Inputs<Message>>,
    pub(crate) outputs: Vec<Outputs<Message>>,
    pub(crate) input_indices: BTreeMap<InputPortId, usize>,
    pub(crate) output_indices: BTreeMap<OutputPortId, usize>,
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

    /// Spawns a block process as a Tokio task and returns its abort handle.
    ///
    /// The `task` future represents an execution of a block and is scheduled
    /// immediately on the active Tokio runtime. [`execute`](Self::execute)
    /// observes its result; an aborted task is reported as [`crate::Error::Join`].
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

    /// Waits for the system's spawned processes, stopping on the first observed failure.
    ///
    /// Returns `Ok(())` when every process succeeds, including for an empty system.
    /// Tasks have already been spawned; their Tokio runtime must remain running
    /// while this future is awaited.
    ///
    /// # Errors
    ///
    /// Returns the first error observed while joining tasks, in completion
    /// order rather than spawn order. Process errors are returned unchanged; task
    /// panics and cancellations are returned as [`crate::Error::Join`].
    ///
    /// On failure, all remaining tasks are aborted and joined before the error
    /// is returned. Additional errors or panics during shutdown are ignored.
    /// Cancellation is cooperative: a task that does not yield can prevent
    /// shutdown from completing. Buffered messages may be discarded on failure.
    ///
    /// # Cancellation
    ///
    /// Dropping this future aborts remaining tasks without waiting for cleanup.
    /// Await it to completion to ensure that all tasks have been joined.
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

/// Validates and prepares a definition as described by [`SystemDefinition::prepare`].
impl TryFrom<&SystemDefinition> for System {
    type Error = SystemPrepareError;

    fn try_from(definition: &SystemDefinition) -> Result<Self, Self::Error> {
        let ports = definition.validated_ports()?;
        let mut connected_inputs = BTreeSet::new();
        for (&(output, input), &type_id) in &definition.connections {
            if !connected_inputs.insert(input) {
                return Err(SystemPrepareError::UnsupportedFanIn(input));
            }
            if type_id != TypeId::of::<Message>() {
                return Err(SystemPrepareError::UnsupportedMessageType {
                    port: output.into(),
                    type_id,
                });
            }
        }

        let mut system = Self::new();
        system.input_indices = ports
            .inputs
            .keys()
            .enumerate()
            .map(|(index, &id)| (id, index))
            .collect();
        system.output_indices = ports
            .outputs
            .keys()
            .enumerate()
            .map(|(index, &id)| (id, index))
            .collect();
        system
            .inputs
            .resize_with(ports.inputs.len(), Inputs::default);
        system
            .outputs
            .resize_with(ports.outputs.len(), Outputs::default);

        // Membership and single-producer/single-consumer constraints were checked
        // before allocation. Index only the validated, dense layout, never raw IDs.
        for &(output, input) in definition.connections.keys() {
            let channel = Channel::<Message>::bounded(1);
            system.outputs[system.output_indices[&output]] = channel.tx;
            system.inputs[system.input_indices[&input]] = channel.rx;
        }
        Ok(system)
    }
}

#[cfg(test)]
mod preparation_tests {
    use super::*;
    use crate::{
        PortEvent, PortState,
        model::{SystemBuilder, SystemValidationError},
    };
    use core::time::Duration;

    fn graph() -> SystemDefinition {
        let mut builder = SystemBuilder::new();
        builder.register_input(InputPortId(-1));
        builder.register_output(OutputPortId(1));
        let mut graph = builder.build();
        graph
            .connections
            .insert((OutputPortId(1), InputPortId(-1)), TypeId::of::<Message>());
        graph
    }

    #[test]
    fn empty_and_one_sided_graphs_prepare_without_a_runtime() {
        let empty = System::try_from(&SystemDefinition::default()).unwrap();
        assert!(empty.inputs.is_empty());
        assert!(empty.outputs.is_empty());
        for inputs in [true, false] {
            let mut builder = SystemBuilder::new();
            if inputs {
                builder.register_input(InputPortId(isize::MIN));
            } else {
                builder.register_output(OutputPortId(isize::MAX));
            }
            let system = builder.build().prepare().unwrap();
            assert_eq!(system.inputs.len(), usize::from(inputs));
            assert_eq!(system.outputs.len(), usize::from(!inputs));
            if inputs {
                assert_eq!(system.inputs[0].state(), PortState::Unconnected);
            } else {
                assert_eq!(system.outputs[0].state(), PortState::Unconnected);
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sparse_extreme_ids_use_dense_storage_and_keep_connections_distinct() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut definition = graph();
            definition.registered_inputs.insert(InputPortId(isize::MIN));
            definition
                .registered_outputs
                .insert(OutputPortId(isize::MAX));
            definition.connections.clear();
            definition.connections.insert(
                (OutputPortId(1), InputPortId(isize::MIN)),
                TypeId::of::<Message>(),
            );
            definition.connections.insert(
                (OutputPortId(isize::MAX), InputPortId(-1)),
                TypeId::of::<Message>(),
            );
            let mut system = definition.prepare().unwrap();
            assert_eq!(system.inputs.len(), 2);
            assert_eq!(system.outputs.len(), 2);
            assert_eq!(system.input_indices.len(), 2);
            assert_eq!(system.output_indices.len(), 2);

            system.outputs[system.output_indices[&OutputPortId(1)]]
                .send_event(PortEvent::Connect)
                .await
                .unwrap();
            system.outputs[system.output_indices[&OutputPortId(isize::MAX)]]
                .send(Message::default())
                .await
                .unwrap();
            system.outputs.clear();
            let first = system.input_indices[&InputPortId(isize::MIN)];
            let second = system.input_indices[&InputPortId(-1)];
            assert!(matches!(
                system.inputs[first].recv_event().await.unwrap(),
                Some(PortEvent::Connect)
            ));
            assert!(matches!(
                system.inputs[second].recv_event().await.unwrap(),
                Some(PortEvent::Message(_))
            ));
            assert!(system.inputs[first].recv_event().await.unwrap().is_none());
            assert!(system.inputs[second].recv_event().await.unwrap().is_none());
        })
        .await
        .expect("prepared channels must drain without hanging");
    }

    #[test]
    fn fan_in_is_rejected_instead_of_overwriting_a_receiver() {
        let mut graph = graph();
        graph.registered_outputs.insert(OutputPortId(2));
        graph
            .connections
            .insert((OutputPortId(2), InputPortId(-1)), TypeId::of::<Message>());
        graph.validate().unwrap();
        assert!(matches!(
            graph.prepare(),
            Err(SystemPrepareError::UnsupportedFanIn(InputPortId(-1)))
        ));
    }

    #[test]
    fn arbitrary_types_are_not_silently_replaced_with_message() {
        let mut graph = graph();
        graph
            .connections
            .insert((OutputPortId(1), InputPortId(-1)), TypeId::of::<u8>());
        graph.validate().unwrap();
        assert!(
            matches!(graph.prepare(), Err(SystemPrepareError::UnsupportedMessageType { type_id, .. }) if type_id == TypeId::of::<u8>())
        );
    }

    #[test]
    fn preparation_revalidates_public_fields_after_edits() {
        let mut graph = graph();
        graph.validate().unwrap();
        graph.inputs.insert(InputPortId(-1), TypeId::of::<u8>());
        let error = graph.prepare().unwrap_err();
        assert!(matches!(
            error,
            SystemPrepareError::InvalidDefinition(SystemValidationError::TypeMismatch {
                port: crate::model::PortId::Input(InputPortId(-1)),
                ..
            })
        ));
        assert!(matches!(
            crate::Error::from(error),
            crate::Error::Prepare(_)
        ));
    }

    #[test]
    fn an_undeclared_id_inside_the_numeric_range_is_still_invalid() {
        let mut graph = graph();
        graph.registered_inputs.insert(InputPortId(-3));
        graph.connections.clear();
        graph
            .connections
            .insert((OutputPortId(1), InputPortId(-2)), TypeId::of::<Message>());
        assert!(matches!(
            graph.prepare(),
            Err(SystemPrepareError::InvalidDefinition(
                SystemValidationError::UnregisteredPort(crate::model::PortId::Input(InputPortId(
                    -2
                )))
            ))
        ));
    }
}
