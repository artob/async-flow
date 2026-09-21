// This is free and unencumbered software released into the public domain.

use async_flow::{
    Cardinality,
    model::{Inputs, Outputs, PortExport, PortRegistration, SystemBuilder, SystemValidationError},
};
use core::{any::TypeId, ops::Bound};

#[test]
fn ranges_validate_intersect_and_add_without_wrapping() {
    assert_eq!(Cardinality::new(2, Some(1)), None);
    assert_eq!(Cardinality::from_limits(-1, 0), Cardinality::UNLIMITED);
    assert_eq!(Cardinality::from_limits(1, 0), Cardinality::ONESHOT);
    assert_eq!(
        Inputs::<u8, 3, 2>::cardinality(),
        (Bound::Included(2), Bound::Included(3))
    );
    assert_eq!(
        Outputs::<u8, -1, 2>::cardinality(),
        (Bound::Included(2), Bound::Unbounded)
    );
    assert_eq!(
        Cardinality::from_limits(5, 2).intersection(Cardinality::from_limits(3, 1)),
        Cardinality::new(2, Some(3))
    );
    assert_eq!(
        Cardinality::from_limits(1, 0).intersection(Cardinality::from_limits(3, 2)),
        None
    );
    assert_eq!(
        Cardinality::ONESHOT.checked_add(Cardinality::ONESHOT),
        Cardinality::new(0, Some(2))
    );
    assert_eq!(
        Cardinality::new(usize::MAX, None)
            .unwrap()
            .checked_add(Cardinality::from_limits(1, 1)),
        None
    );
}

#[test]
fn registration_exports_and_connections_preserve_nondefault_bounds() {
    let input = Inputs::<u8, 3, 2>::default();
    let output = Outputs::<u8, 5, 1>::default();
    let mut builder = SystemBuilder::new();
    builder.register_port(&input);
    builder.register_output(&output);
    builder.export_input(&input).unwrap();
    builder.export(&output).unwrap();
    builder.connect(&output, &input).unwrap();
    let graph = builder.build();
    graph.validate().unwrap();
    assert_eq!(
        graph.cardinalities[&input.id().into()],
        vec![Cardinality::from_limits(3, 2)]
    );
    assert_eq!(
        graph.cardinalities[&output.id().into()],
        vec![Cardinality::from_limits(5, 1)]
    );
}

#[test]
fn typed_export_alone_preserves_bounds_after_raw_registration() {
    let input = Inputs::<u8, 1, 1>::default();
    let output = Outputs::<u8, 1, 1>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(input.id());
    builder.register_port(output.id());
    builder.export_port(&input).unwrap();
    builder.export_output(&output).unwrap();
    let graph = builder.build();
    assert_eq!(
        graph.cardinalities[&input.id().into()],
        vec![Cardinality::from_limits(1, 1)]
    );
    assert_eq!(
        graph.cardinalities[&output.id().into()],
        vec![Cardinality::from_limits(1, 1)]
    );
    graph.validate().unwrap();
}

#[test]
fn connect_preserves_bounds_after_raw_registration_and_defers_disjoint_checks() {
    let input = Inputs::<u8, 1>::default();
    let output = Outputs::<u8, 3, 2>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(input.id());
    builder.register_output(output.id());
    builder.connect(&output, &input).unwrap();
    let graph = builder.build();
    assert!(
        matches!(graph.validate(), Err(SystemValidationError::IncompatibleCardinality { port, .. }) if port == input.id().into())
    );
}

#[test]
fn repeated_constraints_are_intersected_instead_of_overwritten() {
    let input = Inputs::<u8, 1>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(&input);
    builder.register_input(PortRegistration {
        id: input.id(),
        cardinality: Some(Cardinality::from_limits(3, 2)),
    });
    assert!(matches!(
        builder.build().validate(),
        Err(SystemValidationError::IncompatibleCardinality { .. })
    ));

    let mut builder = SystemBuilder::new();
    builder.register_input(&input);
    builder
        .export_input(PortExport {
            id: input.id(),
            type_id: TypeId::of::<u8>(),
            cardinality: Some(Cardinality::from_limits(3, 2)),
        })
        .unwrap();
    assert!(matches!(
        builder.build().validate(),
        Err(SystemValidationError::IncompatibleCardinality { .. })
    ));
}

#[test]
fn cardinality_constraints_require_declared_ports_even_if_the_list_is_empty() {
    let input = Inputs::<u8>::default();
    let mut graph = SystemBuilder::new().build();
    graph.cardinalities.insert(input.id().into(), vec![]);
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::UnregisteredPort(input.id().into()))
    );
}

#[test]
fn fan_in_uses_aggregate_producer_bounds() {
    let input = Inputs::<u8, 2, 2>::default();
    let a = Outputs::<u8, 1, 1>::default();
    let b = Outputs::<u8, 1, 1>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(&input);
    builder.register_output(&a);
    builder.register_output(&b);
    builder.connect(&a, &input).unwrap();
    assert!(builder.clone().build().validate().is_err());
    builder.connect(&b, &input).unwrap();
    builder.build().validate().unwrap();
}

#[test]
fn block_cardinality_metadata_participates_in_validation() {
    use async_flow::model::{BlockDefinition, BlockName, InputPortId, OutputPortId};
    use std::borrow::Cow;
    struct Block(InputPortId, OutputPortId);
    impl BlockName for Block {
        fn name(&self) -> Cow<'_, str> {
            "required".into()
        }
    }
    impl BlockDefinition for Block {
        fn inputs(&self) -> Vec<InputPortId> {
            vec![self.0]
        }
        fn input_cardinality(&self, _: InputPortId) -> Option<Cardinality> {
            Some(Cardinality::from_limits(2, 2))
        }
        fn outputs(&self) -> Vec<OutputPortId> {
            vec![self.1]
        }
        fn output_cardinality(&self, _: OutputPortId) -> Option<Cardinality> {
            Some(Cardinality::from_limits(2, 2))
        }
    }
    let input = Inputs::<u8, 1>::default();
    let output = Outputs::<u8, 1>::default();
    for export_input in [false, true] {
        let mut builder = SystemBuilder::new();
        builder.register(Block(input.id(), output.id()));
        let port = if export_input {
            builder.export_input(&input).unwrap().into()
        } else {
            builder.export_output(&output).unwrap().into()
        };
        assert!(
            matches!(builder.build().validate(), Err(SystemValidationError::IncompatibleCardinality { port: invalid, .. }) if invalid == port)
        );
    }
}

#[test]
fn aggregate_overflow_is_reported_without_rejecting_unbounded_maxima() {
    let input = Inputs::<u8>::default();
    let a = Outputs::<u8>::default();
    let b = Outputs::<u8>::default();
    let c = Outputs::<u8>::default();
    let mut builder = SystemBuilder::new();
    builder.register_input(input.id());
    for output in [&a, &b, &c] {
        builder.register_output(output.id());
        builder.connect(output, &input).unwrap();
    }
    let mut graph = builder.build();
    graph.cardinalities.insert(
        a.id().into(),
        vec![Cardinality::new(0, Some(usize::MAX)).unwrap()],
    );
    graph
        .cardinalities
        .insert(b.id().into(), vec![Cardinality::ONESHOT]);
    graph.validate().unwrap(); // C's unbounded maximum absorbs finite overflow.
    graph.connections.remove(&(c.id(), input.id()));
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::CardinalityOverflow(input.id()))
    );
    graph
        .connections
        .insert((c.id(), input.id()), TypeId::of::<u8>());
    graph.cardinalities.insert(
        a.id().into(),
        vec![Cardinality::new(usize::MAX, None).unwrap()],
    );
    graph
        .cardinalities
        .insert(b.id().into(), vec![Cardinality::from_limits(1, 1)]);
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::CardinalityOverflow(input.id()))
    );
}

#[cfg(feature = "tokio")]
mod runtime {
    use super::*;
    use async_flow::{
        Connection, Error, InputPort, OutputPort, Port, PortEvent, PortState, RecvError, SendError,
        tokio::{Channel, System},
    };
    use core::{
        future::{Future, poll_fn},
        pin::Pin,
        task::Poll,
        time::Duration,
    };
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use tokio::{sync::Barrier, task::JoinSet, time::timeout};

    async fn bounded<T>(future: impl Future<Output = T>) -> T {
        timeout(Duration::from_secs(5), future)
            .await
            .expect("cardinality operation timed out")
    }

    async fn pending<F: Future + ?Sized>(mut future: Pin<&mut F>) {
        poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn oneshot_limits_all_clones_and_finishes_with_live_senders() {
        struct Payload(u8); // Neither Clone nor Default.
        bounded(async {
            let (mut output, mut input) = Channel::<Payload>::oneshot().into_inner();
            let peer = output.clone();
            output.send(Payload(1)).await.unwrap();
            assert_eq!(
                peer.send(Payload(2)).await,
                Err(SendError::CardinalityExceeded { maximum: 1 })
            );
            assert_eq!(input.state(), PortState::Disconnected);
            assert_eq!(output.state(), PortState::Disconnected);
            assert_eq!(input.recv().await.unwrap().unwrap().0, 1);
            assert!(input.recv().await.unwrap().is_none());
            assert!(input.recv_event().await.unwrap().is_none());
            output.close();
            assert_eq!(output.send(Payload(3)).await, Err(SendError::Closed));
            assert_eq!(
                peer.send(Payload(4)).await,
                Err(SendError::CardinalityExceeded { maximum: 1 })
            );
            let (output, mut input) = Channel::<Payload>::oneshot().into_inner();
            drop(output);
            assert!(input.recv().await.unwrap().is_none()); // One-shot permits zero messages.
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn zero_limit_is_an_empty_stream() {
        bounded(async {
            let (output, mut input) = Channel::<u8, 0>::bounded(1).into_inner();
            assert_eq!(
                output.send(1).await,
                Err(SendError::CardinalityExceeded { maximum: 0 })
            );
            assert_eq!(
                output.send_event(PortEvent::Connect).await,
                Err(SendError::CardinalityExceeded { maximum: 0 })
            );
            assert_eq!(input.recv().await.unwrap(), None);
            assert!(input.is_disconnected());
            assert!(output.is_disconnected());
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bounds_are_preserved_by_constructors_and_controls_do_not_spend_quota() {
        bounded(async {
            let channel = Channel::<u8, 2, 2>::bounded(4);
            assert_eq!(Connection::<u8>::type_id(&channel), TypeId::of::<u8>());
            assert_eq!(channel.tx.cardinality(), Cardinality::from_limits(2, 2));
            assert_eq!(channel.rx.cardinality(), Cardinality::from_limits(2, 2));
            let (output, mut input) = channel.into_inner();
            output.send_event(PortEvent::Connect).await.unwrap();
            output.send(1).await.unwrap();
            output.send_event(PortEvent::Connect).await.unwrap();
            output.send(2).await.unwrap();
            assert_eq!(input.recv_event().await.unwrap(), Some(PortEvent::Connect));
            assert_eq!(input.recv().await.unwrap(), Some(1));
            assert_eq!(input.recv().await.unwrap(), Some(2));
            assert_eq!(input.recv_event().await.unwrap(), None);
            for channel in [Channel::<u8, 3, 1>::pair().0, Channel::<u8, 3, 1>::pair().1] {
                assert_eq!(channel.tx.cardinality(), Cardinality::from_limits(3, 1));
                assert_eq!(Connection::<u8>::type_id(&channel), TypeId::of::<u8>());
            }
            assert_eq!(
                Connection::<u8>::type_id(&Channel::<u8>::oneshot()),
                TypeId::of::<u8>()
            );
        })
        .await;
    }

    #[cfg_attr(
        feature = "parallel",
        tokio::test(flavor = "multi_thread", worker_threads = 2)
    )]
    #[cfg_attr(not(feature = "parallel"), tokio::test(flavor = "current_thread"))]
    async fn concurrent_senders_share_quota_and_exhaustion_wakes_capacity_waiters() {
        bounded(async {
            let (output, mut input) = Channel::<usize, 7>::bounded(7).into_inner();
            let barrier = Arc::new(Barrier::new(17));
            let mut tasks = JoinSet::new();
            for id in 0..16 {
                let peer = output.clone();
                let barrier = Arc::clone(&barrier);
                tasks.spawn(async move {
                    barrier.wait().await;
                    (id, peer.send(id).await)
                });
            }
            barrier.wait().await;
            let mut accepted = BTreeSet::new();
            while let Some(result) = tasks.join_next().await {
                let (id, result) = result.unwrap();
                match result {
                    Ok(()) => {
                        accepted.insert(id);
                    },
                    Err(error) => assert_eq!(error, SendError::CardinalityExceeded { maximum: 7 }),
                }
            }
            // No receives until all senders finish: losers must wake despite a full buffer.
            assert_eq!(accepted.len(), 7);
            assert_eq!(
                input
                    .recv_all()
                    .await
                    .unwrap()
                    .into_iter()
                    .collect::<BTreeSet<_>>(),
                accepted
            );
            assert_eq!(input.recv().await.unwrap(), None);
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelling_a_capacity_wait_does_not_spend_message_quota() {
        bounded(async {
            for release_capacity in [false, true] {
                let (output, mut input) = Channel::<u8, 2, 2>::bounded(1).into_inner();
                output.send(1).await.unwrap();
                let mut sending = Box::pin(output.send(2));
                pending(sending.as_mut()).await;
                if release_capacity {
                    assert_eq!(input.recv().await.unwrap(), Some(1));
                }
                drop(sending);
                if !release_capacity {
                    assert_eq!(input.recv().await.unwrap(), Some(1));
                }
                output.send(3).await.unwrap();
                assert_eq!(input.recv().await.unwrap(), Some(3));
                assert_eq!(input.recv().await.unwrap(), None);
            }
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn premature_eof_reports_one_shortfall_for_each_termination_path() {
        bounded(async {
            for end in ["sender", "input", "marker"] {
                let (mut output, mut input) = Channel::<u8, 4, 3>::bounded(3).into_inner();
                output.send(1).await.unwrap();
                output.send(2).await.unwrap();
                match end {
                    "sender" => output.close(),
                    "input" => input.disconnect(),
                    _ => output.send_event(PortEvent::Disconnect).await.unwrap(),
                }
                assert_eq!(
                    input.recv_event().await.unwrap(),
                    Some(PortEvent::Message(1))
                );
                assert_eq!(input.recv().await.unwrap(), Some(2));
                assert_eq!(
                    input.recv_event().await,
                    Err(RecvError::CardinalityUnderflow {
                        minimum: 3,
                        received: 2
                    })
                );
                assert_eq!(input.recv().await.unwrap(), None);
                assert_eq!(input.recv_event().await.unwrap(), None);
            }
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn minimum_counts_received_prefix_not_discarded_tail() {
        bounded(async {
            let (output, mut input) = Channel::<u8, 3, 2>::bounded(3).into_inner();
            output.send(1).await.unwrap();
            output.send_event(PortEvent::Disconnect).await.unwrap();
            output.send(2).await.unwrap();
            assert_eq!(input.recv().await.unwrap(), Some(1));
            assert_eq!(
                input.recv().await,
                Err(RecvError::CardinalityUnderflow {
                    minimum: 2,
                    received: 1
                })
            );
            assert_eq!(input.recv().await.unwrap(), None);
            assert_eq!(output.send(3).await, Err(SendError::Disconnected));
            assert_eq!(output.send(4).await, Err(SendError::Disconnected)); // Failure spent no quota.
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn minimum_is_lifetime_scoped_and_explicit_close_is_abort() {
        bounded(async {
            let (mut output, mut input) = Channel::<u8, -1, 2>::bounded(2).into_inner();
            output.send(1).await.unwrap();
            output.send(2).await.unwrap();
            output.close();
            assert_eq!(
                input.recv_event().await.unwrap(),
                Some(PortEvent::Message(1))
            );
            assert_eq!(input.recv().await.unwrap(), Some(2));
            assert!(input.recv_all().await.unwrap().is_empty());

            let (_, mut input) = Channel::<u8, 2, 2>::bounded(1).into_inner();
            input.close();
            assert_eq!(input.recv().await.unwrap(), None);
            let mut input = async_flow::tokio::Inputs::<u8, 1, 1>::default();
            assert_eq!(
                input.recv().await,
                Err(RecvError::CardinalityUnderflow {
                    minimum: 1,
                    received: 0
                })
            );
            assert_eq!(input.recv().await.unwrap(), None);
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelling_a_receive_does_not_advance_minimum_progress() {
        bounded(async {
            let (mut output, mut input) = Channel::<u8, 2, 2>::bounded(1).into_inner();
            let mut receiving = Box::pin(input.recv());
            pending(receiving.as_mut()).await;
            output.send(1).await.unwrap();
            drop(receiving);
            assert_eq!(input.recv().await.unwrap(), Some(1));
            output.close();
            assert_eq!(
                input.recv().await,
                Err(RecvError::CardinalityUnderflow {
                    minimum: 2,
                    received: 1
                })
            );
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn trait_objects_retain_cardinality_and_lifecycle_operations() {
        bounded(async {
            let (output, input) = Channel::<u8, 3, 1>::bounded(2).into_inner();
            let mut output: Box<dyn OutputPort<u8> + Send> = Box::new(output);
            let mut input: Box<dyn InputPort<u8> + Send> = Box::new(input);
            assert_eq!(output.cardinality(), Some(Cardinality::from_limits(3, 1)));
            assert_eq!(input.cardinality(), output.cardinality());
            assert!(output.is_output());
            assert_eq!(output.capacity(), Some(2));
            output.send(1).await.unwrap();
            input.disconnect();
            assert_eq!(output.send(2).await, Err(SendError::Disconnected));
            assert_eq!(input.recv_all().await.unwrap(), vec![1]);
            output.close();
            input.close();
            assert!(output.is_closed());
            assert!(input.is_closed());
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn shortfalls_propagate_through_system_execution() {
        bounded(async {
            let (output, mut input) = Channel::<u8, 2, 2>::bounded(1).into_inner();
            let result = System::run(|system| {
                system.spawn(async move {
                    output.send(1).await?;
                    Ok(())
                });
                system.spawn(async move {
                    while input.recv().await?.is_some() {}
                    Ok(())
                });
            })
            .await;
            assert!(matches!(
                result,
                Err(Error::Recv(RecvError::CardinalityUnderflow {
                    minimum: 2,
                    received: 1
                }))
            ));
        })
        .await;
    }
}
