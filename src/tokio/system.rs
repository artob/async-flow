// This is free and unencumbered software released into the public domain.

use super::{
    BlockPorts, ChannelFactory, Inputs, Outputs, PortBindingError, ProcessFuture,
    SystemPrepareError,
    runtime_ports::{PortSlot, RuntimePorts},
};
use crate::{
    error::Result,
    io::Message,
    model::{InputPortId, OutputPortId, PortId, SystemDefinition},
};
use alloc::{collections::BTreeMap, vec::Vec};
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
/// Explicit `spawn()` schedules tasks immediately and requires an active runtime.
/// Definition preparation instead stores unpolled process futures; `execute()`
/// starts those futures after every factory has successfully bound its ports.
/// The prepared system is `Send` and does not retain the definition's `Rc` handles.
/// Dropping a system drops unstarted futures and aborts spawned tasks without
/// waiting for cleanup; await `execute()` to join them.
#[derive(Debug, Default)]
pub struct System {
    pub(crate) ports: RuntimePorts,
    pending: Vec<PreparedProcess>,
    pub(crate) blocks: JoinSet<Result>,
}

struct PreparedProcess(ProcessFuture);

impl core::fmt::Debug for PreparedProcess {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("PreparedProcess")
    }
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

    /// Takes the external sender for an exported system input.
    ///
    /// Take boundary handles before `execute()` and drive them concurrently with
    /// execution. Dropping the last external sender allows that input to reach EOF.
    ///
    /// # Errors
    /// Returns a binding error for non-exported, already-taken, or incompatible ports.
    pub fn take_input_sender<T: Send + 'static, const MAX: isize, const MIN: isize>(
        &mut self,
        port: &crate::model::Inputs<T, MAX, MIN>,
    ) -> Result<Outputs<T>, PortBindingError> {
        self.ports.take_input_sender(
            port.id(),
            crate::model::Inputs::<T, MAX, MIN>::message_cardinality(),
        )
    }

    /// Takes the external receiver for an exported system output.
    ///
    /// Receive concurrently with execution to relieve backpressure. Each boundary
    /// receiver can be taken once and retains its effective cardinality constraints.
    ///
    /// # Errors
    /// Returns a binding error for non-exported, already-taken, or incompatible ports.
    pub fn take_output_receiver<T: Send + 'static, const MAX: isize, const MIN: isize>(
        &mut self,
        port: &crate::model::Outputs<T, MAX, MIN>,
    ) -> Result<Inputs<T>, PortBindingError> {
        self.ports.take_output_receiver(
            port.id(),
            crate::model::Outputs::<T, MAX, MIN>::message_cardinality(),
        )
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
    /// Starts prepared processes on the current Tokio runtime. Explicitly spawned
    /// tasks already run on their spawning runtime. Untaken boundary and standalone
    /// endpoints are dropped before prepared startup so they cannot keep streams
    /// alive. Drive taken boundary endpoints concurrently to avoid backpressure
    /// deadlocks. The relevant runtimes must remain running.
    ///
    /// # Errors
    ///
    /// Returns the first error observed while joining tasks, in completion
    /// order rather than spawn order. Process errors are returned unchanged; task
    /// panics and cancellations are returned as [`crate::Error::Join`].
    /// Returns [`crate::Error::Runtime`] if prepared processes need starting but
    /// no active Tokio runtime exists.
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
        if !self.pending.is_empty() {
            let runtime = match tokio::runtime::Handle::try_current() {
                Ok(runtime) => runtime,
                Err(error) => {
                    self.blocks.shutdown().await;
                    return Err(error.into());
                },
            };
            // Only process futures and explicitly taken boundary handles should
            // own endpoints now. Unclaimed handles must not keep streams alive.
            self.ports = RuntimePorts::default();
            for process in core::mem::take(&mut self.pending) {
                self.blocks.spawn_on(process.0, &runtime);
            }
        } else {
            self.ports = RuntimePorts::default();
        }
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
        for (index, block) in definition.blocks.iter().enumerate() {
            if block.executable.is_none() {
                return Err(SystemPrepareError::MissingProcessFactory {
                    index,
                    name: block.name().into_owned(),
                });
            }
        }
        for (&type_id, factory) in &definition.channel_factories {
            if factory.message_type() != type_id {
                return Err(SystemPrepareError::InvalidChannelFactory(type_id));
            }
        }
        let factory = |type_id: TypeId| -> Result<ChannelFactory, SystemPrepareError> {
            definition
                .channel_factories
                .get(&type_id)
                .copied()
                .or_else(|| {
                    (type_id == TypeId::of::<Message>()).then(ChannelFactory::of::<Message>)
                })
                .ok_or(SystemPrepareError::MissingChannelFactory(type_id))
        };
        let mut connections = BTreeMap::<InputPortId, Vec<OutputPortId>>::new();
        for (&(output, input), &type_id) in &definition.connections {
            if definition.inputs.contains(input) {
                return Err(SystemPrepareError::ConnectedExport(input.into()));
            }
            if definition.outputs.contains(output) {
                return Err(SystemPrepareError::ConnectedExport(output.into()));
            }
            factory(type_id)?;
            connections.entry(input).or_default().push(output);
        }
        for (_, &type_id) in definition.inputs.iter() {
            factory(type_id)?;
        }
        for (_, &type_id) in definition.outputs.iter() {
            factory(type_id)?;
        }

        let mut system = Self::new();
        for (&id, &type_id) in &ports.inputs {
            system
                .ports
                .inputs
                .insert(id, PortSlot::new(type_id, ports.cardinality(id.into())));
        }
        for (&id, &type_id) in &ports.outputs {
            system
                .ports
                .outputs
                .insert(id, PortSlot::new(type_id, ports.cardinality(id.into())));
        }
        for (input, outputs) in connections {
            let type_id = definition.connections[&(outputs[0], input)];
            let constructor = factory(type_id)?;
            let bounds = ports.connection_cardinalities[&input];
            if outputs.len() == 1 {
                let (output, receiver) = constructor.single(1, bounds);
                system
                    .ports
                    .outputs
                    .get_mut(&outputs[0])
                    .expect("validated output")
                    .install(output, bounds);
                system
                    .ports
                    .inputs
                    .get_mut(&input)
                    .expect("validated input")
                    .install(receiver, bounds);
            } else {
                let sources: Vec<_> = outputs
                    .into_iter()
                    .map(|id| (id, ports.cardinality(id.into())))
                    .collect();
                let group = constructor.fan_in(1, bounds, &sources)?;
                system
                    .ports
                    .inputs
                    .get_mut(&input)
                    .expect("validated input")
                    .install(group.input, bounds);
                for (id, bounds, output) in group.outputs {
                    system
                        .ports
                        .outputs
                        .get_mut(&id)
                        .expect("validated output")
                        .install(output, bounds);
                }
            }
        }
        for (&id, &type_id) in definition.inputs.iter() {
            let bounds = ports.cardinality(id.into());
            let (sender, input) = factory(type_id)?.single(1, bounds);
            system
                .ports
                .inputs
                .get_mut(&id)
                .expect("validated export")
                .install(input, bounds);
            let mut external = PortSlot::new(Some(type_id), bounds);
            external.install(sender, bounds);
            system.ports.input_senders.insert(id, external);
        }
        for (&id, &type_id) in definition.outputs.iter() {
            let bounds = ports.cardinality(id.into());
            let (output, receiver) = factory(type_id)?.single(1, bounds);
            system
                .ports
                .outputs
                .get_mut(&id)
                .expect("validated export")
                .install(output, bounds);
            let mut external = PortSlot::new(Some(type_id), bounds);
            external.install(receiver, bounds);
            system.ports.output_receivers.insert(id, external);
        }
        for (index, (block, declaration)) in definition.blocks.iter().zip(&ports.blocks).enumerate()
        {
            let mut bindings = BlockPorts {
                ports: &mut system.ports,
                inputs: &declaration.inputs,
                outputs: &declaration.outputs,
            };
            let process = block
                .executable
                .as_ref()
                .expect("validated factory")
                .create_process(&mut bindings)
                .map_err(|error| SystemPrepareError::BlockBinding {
                    index,
                    name: block.name().into_owned(),
                    error,
                })?;
            for port in declaration
                .inputs
                .iter()
                .copied()
                .map(PortId::Input)
                .chain(declaration.outputs.iter().copied().map(PortId::Output))
            {
                let slot = match port {
                    PortId::Input(id) => &system.ports.inputs[&id],
                    PortId::Output(id) => &system.ports.outputs[&id],
                };
                if slot.connected && !slot.claimed {
                    return Err(SystemPrepareError::BlockBinding {
                        index,
                        name: block.name().into_owned(),
                        error: PortBindingError::UnclaimedPort(port),
                    });
                }
            }
            system.pending.push(PreparedProcess(process));
        }
        Ok(system)
    }
}

#[cfg(test)]
mod preparation_tests {
    extern crate std;
    use super::*;
    use crate::{
        Cardinality, PortEvent, PortState,
        model::{SystemBuilder, SystemValidationError},
    };
    use core::time::Duration;

    fn take_pair(system: &mut System) -> (Outputs<Message>, Inputs<Message>) {
        let input = *system.ports.inputs.keys().next().unwrap();
        let output = *system.ports.outputs.keys().next().unwrap();
        (
            system
                .ports
                .take_output(output, Cardinality::UNLIMITED)
                .unwrap(),
            system
                .ports
                .take_input(input, Cardinality::UNLIMITED)
                .unwrap(),
        )
    }

    fn constrained_graph() -> SystemDefinition {
        let input = crate::model::Inputs::<Message, 3, 2>::default();
        let output = crate::model::Outputs::<Message, 5, 1>::default();
        let mut builder = SystemBuilder::new();
        builder.register_input(&input);
        builder.register_output(&output);
        builder.connect(&output, &input).unwrap();
        builder.build()
    }

    #[test]
    fn negotiated_limits_guard_raw_access_even_on_erased_default_types() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let mut system = constrained_graph().prepare().unwrap();
        let (mut output, mut input) = take_pair(&mut system);
        assert_eq!(input.cardinality(), crate::Cardinality::from_limits(3, 2));
        assert_eq!(output.cardinality(), crate::Cardinality::from_limits(3, 2));
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = input.as_ref();
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = input.as_mut();
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = output.as_ref();
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = output.as_mut();
            }))
            .is_err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn preparation_installs_effective_bounds_and_revalidates_cardinality_edits() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut graph = constrained_graph();
            let mut system = graph.prepare().unwrap();
            let (output, mut input) = take_pair(&mut system);
            for _ in 0..3 {
                output.send(Message::default()).await.unwrap();
                assert!(input.recv().await.unwrap().is_some());
            }
            assert_eq!(
                output.send(Message::default()).await,
                Err(crate::SendError::CardinalityExceeded { maximum: 3 })
            );
            assert!(input.recv().await.unwrap().is_none());
            let input_id = *graph.registered_inputs.first().unwrap();
            graph
                .cardinalities
                .entry(input_id.into())
                .or_default()
                .push(crate::Cardinality::from_limits(1, 0));
            assert!(matches!(
                graph.prepare(),
                Err(SystemPrepareError::InvalidDefinition(
                    SystemValidationError::IncompatibleCardinality { .. }
                ))
            ));
        })
        .await
        .expect("prepared cardinality must terminate");
    }

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
        assert!(empty.ports.inputs.is_empty());
        assert!(empty.ports.outputs.is_empty());
        for inputs in [true, false] {
            let mut builder = SystemBuilder::new();
            if inputs {
                builder.register_input(InputPortId(isize::MIN));
            } else {
                builder.register_output(OutputPortId(isize::MAX));
            }
            let mut system = builder.build().prepare().unwrap();
            assert_eq!(system.ports.inputs.len(), usize::from(inputs));
            assert_eq!(system.ports.outputs.len(), usize::from(!inputs));
            if inputs {
                let input = system
                    .ports
                    .take_input::<Message>(InputPortId(isize::MIN), Cardinality::UNLIMITED)
                    .unwrap();
                assert_eq!(input.state(), PortState::Unconnected);
            } else {
                let output = system
                    .ports
                    .take_output::<Message>(OutputPortId(isize::MAX), Cardinality::UNLIMITED)
                    .unwrap();
                assert_eq!(output.state(), PortState::Unconnected);
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
            assert_eq!(system.ports.inputs.len(), 2);
            assert_eq!(system.ports.outputs.len(), 2);
            let first_output = system
                .ports
                .take_output::<Message>(OutputPortId(1), Cardinality::UNLIMITED)
                .unwrap();
            let second_output = system
                .ports
                .take_output::<Message>(OutputPortId(isize::MAX), Cardinality::UNLIMITED)
                .unwrap();
            let mut first = system
                .ports
                .take_input::<Message>(InputPortId(isize::MIN), Cardinality::UNLIMITED)
                .unwrap();
            let mut second = system
                .ports
                .take_input::<Message>(InputPortId(-1), Cardinality::UNLIMITED)
                .unwrap();

            first_output.send_event(PortEvent::Connect).await.unwrap();
            second_output.send(Message::default()).await.unwrap();
            drop(first_output);
            drop(second_output);
            assert!(matches!(
                first.recv_event().await.unwrap(),
                Some(PortEvent::Connect)
            ));
            assert!(matches!(
                second.recv_event().await.unwrap(),
                Some(PortEvent::Message(_))
            ));
            assert!(first.recv_event().await.unwrap().is_none());
            assert!(second.recv_event().await.unwrap().is_none());
        })
        .await
        .expect("prepared channels must drain without hanging");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fan_in_preserves_both_producer_connections() {
        let mut graph = graph();
        graph.registered_outputs.insert(OutputPortId(2));
        graph
            .connections
            .insert((OutputPortId(2), InputPortId(-1)), TypeId::of::<Message>());
        graph.validate().unwrap();
        let mut system = graph.prepare().unwrap();
        let a = system
            .ports
            .take_output::<Message>(OutputPortId(1), Cardinality::UNLIMITED)
            .unwrap();
        let b = system
            .ports
            .take_output::<Message>(OutputPortId(2), Cardinality::UNLIMITED)
            .unwrap();
        let mut input = system
            .ports
            .take_input::<Message>(InputPortId(-1), Cardinality::UNLIMITED)
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            a.send(Message::U64(1)).await.unwrap();
            b.send(Message::U64(2)).await.unwrap();
            drop(a);
            drop(b);
            assert!(matches!(input.recv().await.unwrap(), Some(Message::U64(1))));
            assert!(matches!(input.recv().await.unwrap(), Some(Message::U64(2))));
            assert!(input.recv().await.unwrap().is_none());
        })
        .await
        .unwrap();
    }

    #[test]
    fn arbitrary_types_are_not_silently_replaced_with_message() {
        let mut graph = graph();
        graph
            .connections
            .insert((OutputPortId(1), InputPortId(-1)), TypeId::of::<u8>());
        graph.validate().unwrap();
        assert!(
            matches!(graph.prepare(), Err(SystemPrepareError::MissingChannelFactory(type_id)) if type_id == TypeId::of::<u8>())
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
