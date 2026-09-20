// This is free and unencumbered software released into the public domain.

#![cfg(feature = "tokio")]

use async_flow::{
    Error, Result, SendError,
    tokio::{Channel, System},
};
use core::{
    future::{Future, pending, poll_fn},
    task::Poll,
    time::Duration,
};
use tokio::{sync::oneshot, task::AbortHandle, time::timeout};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    timeout(Duration::from_secs(5), future)
        .await
        .expect("test timed out")
}

fn spawn_pending_block(
    system: &mut System,
) -> (AbortHandle, oneshot::Receiver<()>, oneshot::Receiver<()>) {
    let (started_tx, started_rx) = oneshot::channel();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let abort = system.spawn(async move {
        // Closing this channel proves the task's resources have been dropped.
        let _held_until_cancelled = dropped_tx;
        started_tx.send(()).unwrap();
        pending::<Result>().await
    });
    (abort, started_rx, dropped_rx)
}

#[tokio::test(flavor = "current_thread")]
async fn empty_system_succeeds() {
    bounded(System::new().execute()).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn successful_system_joins_all_blocks_and_drains_messages() {
    let (received_tx, mut received_rx) = oneshot::channel();
    bounded(System::run(|system| {
        let (outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
        system.spawn(async move {
            for value in 0..4 {
                outputs.send(value).await?;
            }
            Ok(())
        });
        system.spawn(async move {
            let mut values = Vec::new();
            while let Some(value) = inputs.recv().await? {
                values.push(value);
            }
            received_tx.send(values).unwrap();
            Ok(())
        });
    }))
    .await
    .unwrap();
    assert_eq!(received_rx.try_recv().unwrap(), vec![0, 1, 2, 3]);
}

#[tokio::test(flavor = "current_thread")]
async fn block_error_aborts_and_joins_remaining_blocks() {
    let mut system = System::new();
    let (_, started, mut dropped) = spawn_pending_block(&mut system);
    system.spawn(async move {
        started.await.unwrap();
        Err(SendError::Closed.into())
    });

    let error = bounded(system.execute()).await.unwrap_err();
    assert!(matches!(error, Error::Send(SendError::Closed)));
    assert_eq!(
        dropped.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn block_panic_is_a_join_error_and_remaining_blocks_are_joined() {
    let mut system = System::new();
    let (_, started, mut dropped) = spawn_pending_block(&mut system);
    system.spawn(async move {
        started.await.unwrap();
        panic!("block panicked");
    });

    let error = bounded(system.execute()).await.unwrap_err();
    let Error::Join(error) = error else {
        panic!("expected a join error, got {error:?}");
    };
    assert!(error.is_panic());
    assert_eq!(
        error.into_panic().downcast_ref::<&str>(),
        Some(&"block panicked")
    );
    assert_eq!(
        dropped.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn aborted_block_is_a_join_error_and_remaining_blocks_are_joined() {
    let mut system = System::new();
    let (abort, started, mut dropped) = spawn_pending_block(&mut system);
    let (_, peer_started, mut peer_dropped) = spawn_pending_block(&mut system);
    bounded(started).await.unwrap();
    bounded(peer_started).await.unwrap();
    abort.abort();

    let error = bounded(system.execute()).await.unwrap_err();
    assert!(matches!(error, Error::Join(error) if error.is_cancelled()));
    assert_eq!(
        dropped.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
    assert_eq!(
        peer_dropped.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_propagates_forwarding_errors() {
    let result = bounded(System::run(|system| {
        let (source, inputs) = Channel::<u8>::bounded(1).into_inner();
        let (outputs, mut sink) = Channel::<u8>::bounded(1).into_inner();
        sink.close();
        system.spawn(async move {
            source.send(1).await?;
            Ok(())
        });
        system.connect(inputs, outputs);
    }))
    .await;
    assert!(matches!(result, Err(Error::Send(SendError::Disconnected))));
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_panic_does_not_replace_the_original_error() {
    struct PanicOnDrop;

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("panic during cleanup");
        }
    }

    let mut system = System::new();
    let (started_tx, started_rx) = oneshot::channel();
    system.spawn(async move {
        let _guard = PanicOnDrop;
        started_tx.send(()).unwrap();
        pending::<Result>().await
    });
    system.spawn(async move {
        started_rx.await.unwrap();
        Err(SendError::Closed.into())
    });

    let error = bounded(system.execute()).await.unwrap_err();
    assert!(matches!(error, Error::Send(SendError::Closed)));
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_execute_aborts_remaining_blocks() {
    let mut system = System::new();
    let (_, started, dropped) = spawn_pending_block(&mut system);
    bounded(started).await.unwrap();

    let mut execute = Box::pin(system.execute());
    poll_fn(|cx| {
        assert!(execute.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(execute);

    bounded(dropped)
        .await
        .expect_err("the cancelled block must drop its resources");
}
