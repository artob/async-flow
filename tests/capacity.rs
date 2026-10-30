// This is free and unencumbered software released into the public domain.

//! Buffer capacities of prepared connections and boundary queues.

#![cfg(feature = "tokio")]

use async_flow::{
    Result,
    model::{
        self, BlockDefinition, BlockName, InputPortId, OutputPortId, PortExport, PortId,
        SystemBuilder, SystemValidationError,
    },
    tokio::{BlockPorts, ExecutableBlock, PortBindingError, ProcessFuture, SystemPrepareError},
};
use core::{future::Future, num::NonZeroUsize, time::Duration};
use std::{
    borrow::Cow,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Notify;

const MESSAGES: usize = 5;

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("execution timed out")
}

fn capacity(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

/// Sends `MESSAGES` messages, counting each one sent.
struct Producer {
    output: model::Outputs<u8>,
    sent: Arc<AtomicUsize>,
}
impl BlockName for Producer {
    fn name(&self) -> Cow<'_, str> {
        "producer".into()
    }
}
impl BlockDefinition for Producer {
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Producer {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let output = ports.take_output(&self.output)?;
        let sent = Arc::clone(&self.sent);
        Ok(Box::pin(async move {
            for n in 0..MESSAGES {
                output.send(n as u8).await?;
                sent.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }))
    }
}

/// Waits for `gate`, then receives everything, counting it.
struct Consumer {
    input: model::Inputs<u8>,
    gate: Arc<Notify>,
    received: Arc<AtomicUsize>,
}
impl BlockName for Consumer {
    fn name(&self) -> Cow<'_, str> {
        "consumer".into()
    }
}
impl BlockDefinition for Consumer {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
}
impl ExecutableBlock for Consumer {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let mut input = ports.take_input(&self.input)?;
        let (gate, received) = (Arc::clone(&self.gate), Arc::clone(&self.received));
        Ok(Box::pin(async move {
            gate.notified().await;
            while input.recv().await?.is_some() {
                received.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }))
    }
}

fn consumer(gate: &Arc<Notify>) -> Consumer {
    Consumer {
        input: Default::default(),
        gate: Arc::clone(gate),
        received: Default::default(),
    }
}

fn producer() -> Producer {
    Producer {
        output: Default::default(),
        sent: Default::default(),
    }
}

/// How many messages each producer had sent while the consumer held back,
/// once the producers stopped making progress; then everything is
/// received once the consumer starts.
async fn sent_while_held_back(capacities: &[Option<usize>]) -> Vec<usize> {
    let gate = Arc::new(Notify::new());
    let mut builder = SystemBuilder::new();
    let consumer = builder.register_executable(consumer(&gate));
    let mut producers = Vec::new();
    for &capacity_of in capacities {
        let producer = builder.register_executable(producer());
        match capacity_of {
            Some(n) => {
                builder.connect_with_capacity(&producer.output, &consumer.input, capacity(n))
            },
            None => builder.connect(&producer.output, &consumer.input),
        }
        .unwrap();
        producers.push(producer);
    }
    let system = builder.build().prepare().unwrap();
    let running = tokio::spawn(system.execute());
    // Let the producers fill their queues; they then wait for the consumer.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let sent = producers
        .iter()
        .map(|producer| producer.sent.load(Ordering::SeqCst))
        .collect();
    gate.notify_one();
    bounded(running).await.unwrap().unwrap();
    assert_eq!(
        consumer.received.load(Ordering::SeqCst),
        MESSAGES * capacities.len(),
        "everything arrives in the end"
    );
    sent
}

#[tokio::test(flavor = "current_thread")]
async fn connections_hold_one_message_unless_given_a_capacity() {
    assert_eq!(sent_while_held_back(&[None]).await, [1]);
    assert_eq!(sent_while_held_back(&[Some(1)]).await, [1]);
    assert_eq!(sent_while_held_back(&[Some(3)]).await, [3]);
    assert_eq!(
        sent_while_held_back(&[Some(MESSAGES)]).await,
        [MESSAGES],
        "a producer can finish before its consumer starts"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn each_fan_in_producer_has_its_own_capacity() {
    assert_eq!(
        sent_while_held_back(&[Some(3), None, Some(2)]).await,
        [3, 1, 2]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn boundary_queues_take_their_exports_capacity() {
    let gate = Arc::new(Notify::new());
    let mut builder = SystemBuilder::new();
    let consumer = builder.register_executable(consumer(&gate));
    builder
        .export_input(PortExport::from(&consumer.input).with_capacity(capacity(3)))
        .unwrap();
    let mut system = builder.build().prepare().unwrap();
    let mut sender = system.take_input_sender(&consumer.input).unwrap();
    let running = tokio::spawn(system.execute());
    // Three go in while nobody receives; the fourth waits.
    for n in 0..3 {
        bounded(sender.send(n)).await.unwrap();
    }
    let fourth = tokio::time::timeout(Duration::from_millis(200), sender.send(3)).await;
    assert!(fourth.is_err(), "the queue holds three");
    gate.notify_one();
    bounded(sender.send(3)).await.unwrap();
    sender.close();
    bounded(running).await.unwrap().unwrap();
    assert_eq!(consumer.received.load(Ordering::SeqCst), 4);
}

#[test]
fn misplaced_or_oversized_capacities_are_rejected() {
    let gate = Arc::new(Notify::new());
    let build = || {
        let mut builder = SystemBuilder::new();
        let producer = builder.register_executable(producer());
        let consumer = builder.register_executable(consumer(&gate));
        builder.connect(&producer.output, &consumer.input).unwrap();
        (builder.build(), producer, consumer)
    };

    // A connected input's queues are its producers'.
    let (mut definition, _, consumer) = build();
    definition
        .capacities
        .insert(consumer.input.id().into(), capacity(2));
    assert_eq!(
        definition.validate(),
        Err(SystemValidationError::UnexportedInputCapacity(
            consumer.input.id()
        ))
    );

    let (mut definition, ..) = build();
    // A port no block declares.
    let stranger: PortId = model::Outputs::<u8>::default().id().into();
    definition.capacities.insert(stranger, capacity(2));
    assert_eq!(
        definition.validate(),
        Err(SystemValidationError::UnregisteredPort(stranger))
    );

    let (mut definition, producer, _) = build();
    let port = producer.output.id().into();
    definition.capacities.insert(port, capacity(usize::MAX));
    assert!(matches!(
        definition.prepare(),
        Err(SystemPrepareError::CapacityTooLarge { port: p, capacity: c })
            if p == port && c == usize::MAX
    ));
}
