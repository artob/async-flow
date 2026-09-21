// This is free and unencumbered software released into the public domain.

#![cfg(feature = "tokio")]

use async_flow::{
    Error, Result, SendError,
    model::{self, BlockDefinition, BlockName, InputPortId, OutputPortId, SystemBuilder},
    tokio::{
        BlockPorts, ChannelFactory, ExecutableBlock, PortBindingError, ProcessFuture,
        SystemPrepareError,
    },
};
use core::{
    any::TypeId,
    future::{Future, pending},
    time::Duration,
};
use std::{
    borrow::Cow,
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("execution timed out")
}

struct Source {
    output: model::Outputs<u8, 2, 2>,
    polled: Arc<AtomicUsize>,
}
impl BlockName for Source {
    fn name(&self) -> Cow<'_, str> {
        "source".into()
    }
}
impl BlockDefinition for Source {
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Source {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let output = ports.take_output(&self.output)?;
        let polled = Arc::clone(&self.polled);
        Ok(Box::pin(async move {
            polled.fetch_add(1, Ordering::SeqCst);
            output.send(4).await?;
            output.send(9).await?;
            Ok(())
        }))
    }
}

struct Format {
    input: model::Inputs<u8>,
    output: model::Outputs<String>,
    factories: Rc<Cell<usize>>,
}
impl BlockName for Format {
    fn name(&self) -> Cow<'_, str> {
        "format".into()
    }
}
impl BlockDefinition for Format {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Format {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        self.factories.set(self.factories.get() + 1);
        let mut input = ports.take_input(&self.input)?;
        let output = ports.take_output(&self.output)?;
        Ok(Box::pin(async move {
            while let Some(value) = input.recv().await? {
                output.send(value.to_string()).await?;
            }
            Ok(())
        }))
    }
}

#[test]
fn preparation_is_runtime_free_repeatable_sendable_and_unpolled() {
    let polled = Arc::new(AtomicUsize::new(0));
    let factories = Rc::new(Cell::new(0));
    let mut builder = SystemBuilder::new();
    let source = builder.register_executable(Source {
        output: Default::default(),
        polled: Arc::clone(&polled),
    });
    let format = builder.register_executable(Format {
        input: Default::default(),
        output: Default::default(),
        factories: Rc::clone(&factories),
    });
    builder.connect(&source.output, &format.input).unwrap();
    builder.export_output(&format.output).unwrap();
    let mut definition = builder.build();
    definition.blocks.reverse(); // Factories travel with handles, not old positions.
    let mut first = definition.prepare().unwrap();
    let first_rx = first.take_output_receiver(&format.output).unwrap();
    let mut second = definition.prepare().unwrap();
    let second_rx = second.take_output_receiver(&format.output).unwrap();
    assert_eq!(factories.get(), 2);
    assert_eq!(polled.load(Ordering::SeqCst), 0);
    drop(definition);
    drop(source);
    drop(format); // Runtime state owns no Rc metadata.
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(bounded(async {
                for (system, mut input) in [(first, first_rx), (second, second_rx)] {
                    let collect = async {
                        let mut values = Vec::new();
                        while let Some(value) = input.recv().await? {
                            values.push(value);
                        }
                        Ok::<_, Error>(values)
                    };
                    let (_, values) = tokio::try_join!(system.execute(), collect).unwrap();
                    assert_eq!(values, vec!["4", "9"]);
                }
            }));
    })
    .join()
    .unwrap();
    assert_eq!(polled.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn executable_producers_fan_in_to_a_typed_transformer() {
    bounded(async {
        let mut builder = SystemBuilder::new();
        let a = builder.register_executable(Source {
            output: Default::default(),
            polled: Arc::new(AtomicUsize::new(0)),
        });
        let b = builder.register_executable(Source {
            output: Default::default(),
            polled: Arc::new(AtomicUsize::new(0)),
        });
        let format = builder.register_executable(Format {
            input: Default::default(),
            output: Default::default(),
            factories: Rc::new(Cell::new(0)),
        });
        builder.connect(&a.output, &format.input).unwrap();
        builder.connect(&b.output, &format.input).unwrap();
        builder.export_output(&format.output).unwrap();
        let mut system = builder.build().prepare().unwrap();
        let mut receiver = system.take_output_receiver(&format.output).unwrap();
        let collect = async {
            let mut values = Vec::new();
            while let Some(value) = receiver.recv().await? {
                values.push(value);
            }
            Ok::<_, Error>(values)
        };
        let (_, mut values) = tokio::try_join!(system.execute(), collect).unwrap();
        values.sort();
        assert_eq!(values, vec!["4", "4", "9", "9"]);
    })
    .await;
}

struct Echo {
    input: model::Inputs<u8>,
    output: model::Outputs<u8>,
}
impl BlockName for Echo {
    fn name(&self) -> Cow<'_, str> {
        "echo".into()
    }
}
impl BlockDefinition for Echo {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Echo {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let mut input = ports.take_input(&self.input)?;
        let output = ports.take_output(&self.output)?;
        Ok(Box::pin(async move {
            while let Some(value) = input.recv().await? {
                output.send(value).await?;
            }
            Ok(())
        }))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn boundary_endpoints_are_single_owner_and_driven_concurrently() {
    bounded(async {
        let mut builder = SystemBuilder::new();
        let echo = builder.register_executable(Echo {
            input: Default::default(),
            output: Default::default(),
        });
        builder.export_input(&echo.input).unwrap();
        builder.export_output(&echo.output).unwrap();
        let definition = builder.build();
        let mut system = definition.prepare().unwrap();
        let sender = system.take_input_sender(&echo.input).unwrap();
        assert!(matches!(
            system.take_input_sender(&echo.input),
            Err(PortBindingError::AlreadyClaimed(_))
        ));
        let mut receiver = system.take_output_receiver(&echo.output).unwrap();
        assert!(matches!(
            system.take_output_receiver(&echo.output),
            Err(PortBindingError::AlreadyClaimed(_))
        ));
        let io = async move {
            let sending = async move {
                for n in 0..5 {
                    sender.send(n).await?;
                }
                Ok::<_, Error>(())
            };
            let receiving = async move {
                let mut values = Vec::new();
                while let Some(n) = receiver.recv().await? {
                    values.push(n);
                }
                Ok::<_, Error>(values)
            };
            let (_, values) = tokio::try_join!(sending, receiving)?;
            Ok::<_, Error>(values)
        };
        let (_, values) = tokio::try_join!(system.execute(), io).unwrap();
        assert_eq!(values, vec![0, 1, 2, 3, 4]);
        // Untaken boundary senders must not keep this process alive.
        definition.prepare().unwrap().execute().await.unwrap();
    })
    .await;
}

struct FailingFactory;
impl BlockName for FailingFactory {
    fn name(&self) -> Cow<'_, str> {
        "failure".into()
    }
}
impl BlockDefinition for FailingFactory {}
impl ExecutableBlock for FailingFactory {
    fn create_process(&self, _: &mut BlockPorts<'_>) -> Result<ProcessFuture, PortBindingError> {
        Err(PortBindingError::InvalidConfiguration("bad config".into()))
    }
}

struct Guard(Arc<AtomicUsize>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
struct Pending {
    polled: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}
impl BlockName for Pending {
    fn name(&self) -> Cow<'_, str> {
        "pending".into()
    }
}
impl BlockDefinition for Pending {}
impl ExecutableBlock for Pending {
    fn create_process(&self, _: &mut BlockPorts<'_>) -> Result<ProcessFuture, PortBindingError> {
        let guard = Guard(Arc::clone(&self.dropped));
        let polled = Arc::clone(&self.polled);
        Ok(Box::pin(async move {
            let _guard = guard;
            polled.fetch_add(1, Ordering::SeqCst);
            pending::<Result>().await
        }))
    }
}

#[test]
fn factory_failure_and_dropping_prepared_system_release_unpolled_futures() {
    let polled = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let mut builder = SystemBuilder::new();
    builder.register_executable(Pending {
        polled: Arc::clone(&polled),
        dropped: Arc::clone(&dropped),
    });
    drop(builder.clone().build().prepare().unwrap());
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    builder.register_executable(FailingFactory);
    assert!(matches!(
        builder.build().prepare(),
        Err(SystemPrepareError::BlockBinding { index: 1, .. })
    ));
    assert_eq!(polled.load(Ordering::SeqCst), 0);
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
}

struct Failure {
    panic: bool,
}
impl BlockName for Failure {
    fn name(&self) -> Cow<'_, str> {
        "failure".into()
    }
}
impl BlockDefinition for Failure {}
impl ExecutableBlock for Failure {
    fn create_process(&self, _: &mut BlockPorts<'_>) -> Result<ProcessFuture, PortBindingError> {
        let panic = self.panic;
        Ok(Box::pin(async move {
            if panic {
                panic!("process panic");
            }
            Err(SendError::Closed.into())
        }))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn prepared_process_failures_abort_and_join_peers() {
    bounded(async {
        for panic in [false, true] {
            let dropped = Arc::new(AtomicUsize::new(0));
            let mut builder = SystemBuilder::new();
            builder.register_executable(Pending {
                polled: Arc::new(AtomicUsize::new(0)),
                dropped: Arc::clone(&dropped),
            });
            builder.register_executable(Failure { panic });
            let result = builder.build().prepare().unwrap().execute().await;
            if panic {
                assert!(matches!(result, Err(Error::Join(error)) if error.is_panic()));
            } else {
                assert!(matches!(result, Err(Error::Send(SendError::Closed))));
            }
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
        }
    })
    .await;
}

#[test]
fn executing_prepared_processes_without_runtime_returns_error() {
    use core::{
        pin::pin,
        task::{Context, Poll, Waker},
    };
    let mut builder = SystemBuilder::new();
    builder.register_executable(Failure { panic: false });
    let system = builder.build().prepare().unwrap();
    let mut execute = pin!(system.execute());
    assert!(matches!(
        execute
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(Error::Runtime(_)))
    ));
}

#[test]
fn malformed_runtime_metadata_fails_before_any_factory_runs() {
    let factories = Rc::new(Cell::new(0));
    let mut builder = SystemBuilder::new();
    let block = builder.register_executable(Format {
        input: Default::default(),
        output: Default::default(),
        factories: Rc::clone(&factories),
    });
    builder.export_output(&block.output).unwrap();
    let mut definition = builder.build();
    definition.channel_factories.clear();
    assert!(
        matches!(definition.prepare(), Err(SystemPrepareError::MissingChannelFactory(id)) if id == TypeId::of::<String>())
    );
    definition
        .channel_factories
        .insert(TypeId::of::<String>(), ChannelFactory::of::<u8>());
    assert!(matches!(
        definition.prepare(),
        Err(SystemPrepareError::InvalidChannelFactory(_))
    ));
    assert_eq!(factories.get(), 0);
}

#[test]
fn connected_exports_are_rejected_in_both_directions() {
    for input_export in [false, true] {
        let input = model::Inputs::<u8>::default();
        let output = model::Outputs::<u8>::default();
        let mut builder = SystemBuilder::new();
        builder.register_input(&input);
        builder.register_output(&output);
        builder.connect(&output, &input).unwrap();
        if input_export {
            builder.export_input(&input).unwrap();
        } else {
            builder.export_output(&output).unwrap();
        }
        assert!(matches!(
            builder.build().prepare(),
            Err(SystemPrepareError::ConnectedExport(_))
        ));
    }
}

struct BadBinding {
    input: model::Inputs<u8>,
    foreign: model::Inputs<u8>,
    mode: u8,
}
impl BlockName for BadBinding {
    fn name(&self) -> Cow<'_, str> {
        "bad binding".into()
    }
}
impl BlockDefinition for BadBinding {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
    fn input_type(&self, _: InputPortId) -> Option<TypeId> {
        (self.mode == 2).then(TypeId::of::<String>)
    }
}
impl ExecutableBlock for BadBinding {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        if self.mode == 0 {
            let _ = ports.take_input(&self.foreign)?;
        }
        if self.mode == 1 {
            let _ = ports.take_input(&self.input)?;
            let _ = ports.take_input(&self.input)?;
        }
        if self.mode == 2 {
            let _ = ports.take_input(&self.input)?;
        }
        Ok(Box::pin(async { Ok(()) }))
    }
}

#[test]
fn scoped_bindings_reject_foreign_duplicate_wrong_type_and_unclaimed_ports() {
    for mode in 0..4 {
        let mut builder = SystemBuilder::new();
        let block = builder.register_executable(BadBinding {
            input: Default::default(),
            foreign: Default::default(),
            mode,
        });
        if mode == 3 {
            builder.export_input(&block.input).unwrap();
        }
        let error = builder.build().prepare().unwrap_err();
        let SystemPrepareError::BlockBinding { error, .. } = error else {
            panic!("{error:?}");
        };
        match mode {
            0 => assert!(matches!(error, PortBindingError::ForeignPort(_))),
            1 => assert!(matches!(error, PortBindingError::AlreadyClaimed(_))),
            2 => assert!(matches!(error, PortBindingError::TypeMismatch { .. })),
            _ => assert!(matches!(error, PortBindingError::UnclaimedPort(_))),
        }
    }
}

#[test]
fn metadata_only_blocks_require_explicit_executable_registration() {
    let mut builder = SystemBuilder::new();
    builder.register(Failure { panic: false });
    let definition = builder.build();
    definition.validate().unwrap();
    assert!(matches!(
        definition.prepare(),
        Err(SystemPrepareError::MissingProcessFactory { index: 0, .. })
    ));
}

struct Sink {
    input: model::Inputs<String>,
    values: Arc<std::sync::Mutex<Vec<String>>>,
}
impl BlockName for Sink {
    fn name(&self) -> Cow<'_, str> {
        "sink".into()
    }
}
impl BlockDefinition for Sink {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
}
impl ExecutableBlock for Sink {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let mut input = ports.take_input(&self.input)?;
        let values = Arc::clone(&self.values);
        Ok(Box::pin(async move {
            while let Some(value) = input.recv().await? {
                values.lock().unwrap().push(value);
            }
            Ok(())
        }))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn closed_pipeline_executes_typed_producer_transformer_and_consumer() {
    bounded(async {
        let values = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut builder = SystemBuilder::new();
        let source = builder.register_executable(Source {
            output: Default::default(),
            polled: Arc::new(AtomicUsize::new(0)),
        });
        let format = builder.register_executable(Format {
            input: Default::default(),
            output: Default::default(),
            factories: Rc::new(Cell::new(0)),
        });
        let sink = builder.register_executable(Sink {
            input: Default::default(),
            values: Arc::clone(&values),
        });
        builder.connect(&source.output, &format.input).unwrap();
        builder.connect(&format.output, &sink.input).unwrap();
        builder.build().prepare().unwrap().execute().await.unwrap();
        assert_eq!(*values.lock().unwrap(), vec!["4", "9"]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_execution_aborts_prepared_processes() {
    bounded(async {
        let polled = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut builder = SystemBuilder::new();
        builder.register_executable(Pending {
            polled: Arc::clone(&polled),
            dropped: Arc::clone(&dropped),
        });
        let execution = tokio::spawn(builder.build().prepare().unwrap().execute());
        while polled.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        execution.abort();
        assert!(execution.await.unwrap_err().is_cancelled());
        while dropped.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn raw_export_metadata_uses_explicit_registered_constructors() {
    bounded(async {
        let mut builder = SystemBuilder::new();
        let block = builder.register_executable(Echo {
            input: Default::default(),
            output: Default::default(),
        });
        builder
            .export_input((block.input.id(), TypeId::of::<u8>()))
            .unwrap();
        builder
            .export_output((block.output.id(), TypeId::of::<u8>()))
            .unwrap();
        assert!(matches!(
            builder.clone().build().prepare(),
            Err(SystemPrepareError::MissingChannelFactory(_))
        ));
        builder.register_message_type::<u8>();
        let mut system = builder.build().prepare().unwrap();
        let sender = system.take_input_sender(&block.input).unwrap();
        let mut receiver = system.take_output_receiver(&block.output).unwrap();
        let drive = async {
            sender.send(42).await?;
            drop(sender);
            assert_eq!(receiver.recv().await?, Some(42));
            assert_eq!(receiver.recv().await?, None);
            Ok::<_, Error>(())
        };
        tokio::try_join!(system.execute(), drive).unwrap();
    })
    .await;
}

#[test]
fn bound_failures_do_not_remove_the_requested_boundary() {
    let input = model::Inputs::<u8>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(&input);
    // Raw metadata contradicts the caller's descriptor but is structurally valid.
    builder
        .export_input((input.id(), TypeId::of::<String>()))
        .unwrap();
    builder.register_message_type::<String>();
    let mut system = builder.build().prepare().unwrap();
    for _ in 0..2 {
        assert!(matches!(
            system.take_input_sender(&input),
            Err(PortBindingError::TypeMismatch { .. })
        ));
    }
    let other = model::Inputs::<u8>::default();
    assert!(matches!(
        system.take_input_sender(&other),
        Err(PortBindingError::NotExported(_))
    ));
}

#[test]
fn prepared_message_alias_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<async_flow::Message>();
}

#[test]
fn factories_cannot_silently_narrow_already_connected_port_bounds() {
    let mut builder = SystemBuilder::new();
    let source = builder.register_executable(Source {
        output: Default::default(),
        polled: Arc::new(AtomicUsize::new(0)),
    });
    // The raw export omitted the descriptor's exactly-two metadata.
    builder
        .export_output((source.output.id(), TypeId::of::<u8>()))
        .unwrap();
    builder.register_message_type::<u8>();
    assert!(matches!(
        builder.build().prepare(),
        Err(SystemPrepareError::BlockBinding {
            error: PortBindingError::CardinalityMismatch { .. },
            ..
        })
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn payloads_need_send_but_not_clone_default_or_sync() {
    struct Payload(Cell<u8>);
    struct Producer(model::Outputs<Payload, 1, 1>);
    impl BlockName for Producer {
        fn name(&self) -> Cow<'_, str> {
            "opaque".into()
        }
    }
    impl BlockDefinition for Producer {
        fn outputs(&self) -> Vec<OutputPortId> {
            vec![self.0.id()]
        }
    }
    impl ExecutableBlock for Producer {
        fn create_process(
            &self,
            ports: &mut BlockPorts<'_>,
        ) -> Result<ProcessFuture, PortBindingError> {
            let output = ports.take_output(&self.0)?;
            Ok(Box::pin(async move {
                output.send(Payload(Cell::new(7))).await?;
                Ok(())
            }))
        }
    }
    bounded(async {
        let mut builder = SystemBuilder::new();
        let producer = builder.register_executable(Producer(Default::default()));
        builder.export_output(&producer.0).unwrap();
        let mut system = builder.build().prepare().unwrap();
        let mut output = system.take_output_receiver(&producer.0).unwrap();
        let collect = async {
            assert_eq!(output.recv().await?.unwrap().0.get(), 7);
            assert!(output.recv().await?.is_none());
            Ok::<_, Error>(())
        };
        tokio::try_join!(system.execute(), collect).unwrap();
    })
    .await;
}
