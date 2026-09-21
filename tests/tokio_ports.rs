// This is free and unencumbered software released into the public domain.

#![cfg(feature = "tokio")]

use async_flow::{
    InputPort, OutputPort, Port, PortDirection, PortEvent, PortState, SendError,
    tokio::{Channel, Inputs, Outputs},
};
use core::{
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
    time::Duration,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    timeout(Duration::from_secs(5), future)
        .await
        .expect("port operation timed out")
}

async fn assert_pending<F: Future + ?Sized>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending(), "operation must wait");
        Poll::Ready(())
    })
    .await;
}

#[derive(Debug)]
struct Tracked {
    value: u8,
    drops: Arc<AtomicUsize>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn tracked(value: u8, drops: &Arc<AtomicUsize>) -> Tracked {
    Tracked {
        value,
        drops: Arc::clone(drops),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn disconnect_event_is_terminal_for_both_receive_apis() {
    bounded(async {
        for receive_events in [false, true] {
            for disconnect_first in [false, true] {
                let (outputs, mut inputs) = Channel::<u8>::bounded(4).into_inner();
                let peer = outputs.clone();
                outputs.send(1).await.unwrap();
                outputs.send_event(PortEvent::Disconnect).await.unwrap();
                peer.send_event(PortEvent::Connect).await.unwrap();
                peer.send(2).await.unwrap();
                assert_eq!(inputs.state(), PortState::Connected);
                if disconnect_first {
                    inputs.disconnect();
                }
                assert_eq!(inputs.recv().await.unwrap(), Some(1));

                if receive_events {
                    assert_eq!(
                        inputs.recv_event().await.unwrap(),
                        Some(PortEvent::Disconnect)
                    );
                } else {
                    assert_eq!(inputs.recv().await.unwrap(), None);
                }
                assert_eq!(inputs.recv_event().await.unwrap(), None);
                assert_eq!(inputs.recv().await.unwrap(), None);
                assert_eq!(inputs.state(), PortState::Disconnected);
                assert!(inputs.is_empty());
                assert_eq!(inputs.capacity(), None);
                assert_eq!(outputs.state(), PortState::Disconnected);
                assert_eq!(peer.state(), PortState::Disconnected);
                assert_eq!(peer.send(3).await, Err(SendError::Disconnected));
                assert_eq!(
                    peer.send_event(PortEvent::Connect).await,
                    Err(SendError::Disconnected)
                );

                inputs.disconnect();
                assert_eq!(inputs.state(), PortState::Disconnected);
                inputs.close();
                inputs.close();
                assert_eq!(inputs.state(), PortState::Closed);
                assert_eq!(inputs.recv_event().await.unwrap(), None);
            }
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn unconnected_and_closed_endpoints_have_stable_states() {
    bounded(async {
        let mut inputs = Inputs::<u8>::default();
        let mut outputs = Outputs::<u8>::default();
        assert!(inputs.is_input());
        assert!(outputs.is_output());
        assert_eq!(inputs.direction(), PortDirection::Input);
        assert_eq!(outputs.direction(), PortDirection::Output);
        assert!(inputs.is_unconnected());
        assert!(outputs.is_unconnected());
        assert_eq!(Port::capacity(&inputs), None);
        assert_eq!(Port::capacity(&outputs), None);
        assert!(inputs.is_empty());
        assert_eq!(inputs.recv().await.unwrap(), None);
        assert_eq!(inputs.recv_event().await.unwrap(), None);
        inputs.disconnect();
        assert!(inputs.is_unconnected());
        assert_eq!(outputs.send(1).await, Err(SendError::Unconnected));
        assert_eq!(
            outputs.send_event(PortEvent::Connect).await,
            Err(SendError::Unconnected)
        );
        Port::close(&mut inputs);
        Port::close(&mut inputs);
        inputs.disconnect();
        Port::close(&mut outputs);
        Port::close(&mut outputs);
        assert!(inputs.is_closed());
        assert!(outputs.is_closed());
        assert_eq!(inputs.recv_event().await.unwrap(), None);
        assert_eq!(inputs.recv().await.unwrap(), None);
        assert_eq!(outputs.send(1).await, Err(SendError::Closed));
        assert_eq!(
            outputs.send_event(PortEvent::Disconnect).await,
            Err(SendError::Closed)
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn controls_consume_capacity_and_backpressure_is_visible_through_port_traits() {
    bounded(async {
        let (outputs, mut inputs) = Channel::<u8>::bounded(2).into_inner();
        assert!(inputs.is_empty()); // No synthetic Connect event.
        assert_eq!(Port::max_capacity(&inputs), Some(2));
        assert_eq!(Port::max_capacity(&outputs), Some(2));
        outputs.send_event(PortEvent::Connect).await.unwrap();
        outputs.send_event(PortEvent::Connect).await.unwrap();
        assert_eq!(Port::capacity(&inputs), Some(0));
        assert_eq!(Port::capacity(&outputs), Some(0));
        let mut sending = Box::pin(outputs.send(7));
        assert_pending(sending.as_mut()).await;
        assert_eq!(inputs.recv_event().await.unwrap(), Some(PortEvent::Connect));
        sending.await.unwrap();
        assert_eq!(Port::capacity(&outputs), Some(0));
        assert_eq!(inputs.recv().await.unwrap(), Some(7)); // Filters the other Connect.
        assert_eq!(Port::capacity(&inputs), Some(2));
        assert_eq!(Port::capacity(&outputs), Some(2));
        assert!(inputs.is_connected());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn input_disconnect_drains_accepted_events_and_connect_does_not_reopen_it() {
    bounded(async {
        let (outputs, mut inputs) = Channel::<u8>::bounded(3).into_inner();
        let peer = outputs.clone();
        outputs.send_event(PortEvent::Connect).await.unwrap();
        outputs.send(1).await.unwrap();
        peer.send(2).await.unwrap();
        inputs.disconnect();
        inputs.disconnect();
        assert!(inputs.is_disconnected());
        assert!(!inputs.is_empty());
        assert!(outputs.is_disconnected());
        assert!(peer.is_disconnected());
        assert_eq!(outputs.send(3).await, Err(SendError::Disconnected));
        assert_eq!(
            OutputPort::send(&peer, 3).await,
            Err(SendError::Disconnected)
        );
        assert_eq!(inputs.recv_event().await.unwrap(), Some(PortEvent::Connect));
        assert!(inputs.is_disconnected());
        let mut input: Box<dyn InputPort<u8> + Send> = Box::new(inputs);
        assert_eq!(input.max_capacity(), Some(3));
        assert_eq!(input.recv_all().await.unwrap(), vec![1, 2]);
        assert!(input.is_disconnected());
        assert!(!input.is_closed());
        assert_eq!(input.capacity(), Some(3)); // Natural EOF retains the receiver.
        assert_eq!(input.recv().await.unwrap(), None);
        input.close();
        assert!(input.is_closed());
        assert_eq!(input.capacity(), None);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn output_close_is_handle_local_and_last_sender_closure_drains_to_eof() {
    bounded(async {
        let (mut outputs, inputs) = Channel::<u8>::bounded(3).into_inner();
        let mut peer = outputs.clone();
        let mut inputs: Box<dyn InputPort<u8> + Send> = Box::new(inputs);
        outputs.send(1).await.unwrap();
        peer.send(2).await.unwrap();
        Port::close(&mut outputs);
        Port::close(&mut outputs);
        assert!(outputs.is_closed());
        assert_eq!(Port::capacity(&outputs), None);
        assert!(peer.is_connected());
        assert!(inputs.is_connected());
        assert_eq!(outputs.send(3).await, Err(SendError::Closed));
        assert!(outputs.clone().is_closed());
        peer.send(3).await.unwrap();
        peer.close();
        assert!(inputs.is_disconnected());
        assert_eq!(inputs.recv_all().await.unwrap(), vec![1, 2, 3]);
        assert_eq!(inputs.recv().await.unwrap(), None);
        assert_eq!(inputs.recv().await.unwrap(), None);
        assert!(inputs.is_disconnected());
        assert_eq!(inputs.capacity(), Some(3));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn receiver_shutdown_wakes_blocked_sends_and_obeys_buffer_retention() {
    bounded(async {
        for shutdown in ["disconnect", "close", "disconnect_then_close", "drop"] {
            let drops = Arc::new(AtomicUsize::new(0));
            let (outputs, inputs) = Channel::<Tracked>::bounded(1).into_inner();
            let mut inputs = Some(inputs);
            outputs.send(tracked(1, &drops)).await.unwrap();
            let peer = Outputs::<Tracked>::from(outputs.as_ref());
            let message = tracked(2, &drops);
            let (waiting_tx, waiting_rx) = oneshot::channel();
            let sending = tokio::spawn(async move {
                let mut sending = Box::pin(peer.send(message));
                assert_pending(sending.as_mut()).await;
                waiting_tx.send(()).unwrap();
                sending.await
            });
            waiting_rx.await.unwrap();
            match shutdown {
                "disconnect" => inputs.as_mut().unwrap().disconnect(),
                "close" => {
                    inputs.as_mut().unwrap().close();
                    inputs.as_mut().unwrap().close();
                },
                "disconnect_then_close" => {
                    let input = inputs.as_mut().unwrap();
                    input.disconnect();
                    assert_eq!(drops.load(Ordering::SeqCst), 0);
                    input.close();
                },
                _ => drop(inputs.take()),
            }
            assert_eq!(sending.await.unwrap(), Err(SendError::Disconnected));
            assert!(outputs.is_disconnected());
            if shutdown == "disconnect" {
                assert_eq!(drops.load(Ordering::SeqCst), 1);
                let input = inputs.as_mut().unwrap();
                let message = input.recv().await.unwrap().unwrap();
                assert_eq!(message.value, 1);
                drop(message);
                assert!(input.recv().await.unwrap().is_none());
            } else if let Some(input) = inputs.as_mut() {
                assert!(input.is_closed());
                assert!(input.is_empty());
                assert!(input.recv_event().await.unwrap().is_none());
            }
            assert_eq!(drops.load(Ordering::SeqCst), 2, "{shutdown}");
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn terminal_disconnect_drops_buffered_payloads_after_the_marker() {
    bounded(async {
        let drops = Arc::new(AtomicUsize::new(0));
        let (outputs, mut inputs) = Channel::<Tracked>::bounded(3).into_inner();
        outputs.send(tracked(1, &drops)).await.unwrap();
        outputs.send_event(PortEvent::Disconnect).await.unwrap();
        outputs.send(tracked(2, &drops)).await.unwrap();
        let message = inputs.recv().await.unwrap().unwrap();
        assert_eq!(message.value, 1);
        drop(message);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(inputs.recv().await.unwrap().is_none());
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert!(inputs.recv_event().await.unwrap().is_none());
        assert!(inputs.is_disconnected());
        assert!(!inputs.is_closed());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn disconnect_waits_for_previously_reserved_capacity() {
    bounded(async {
        for deliver in [false, true] {
            let (outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
            let permit = outputs.as_ref().reserve().await.unwrap();
            assert_eq!(Port::capacity(&inputs), Some(0));
            assert_eq!(Port::capacity(&outputs), Some(0));
            assert!(inputs.is_empty());
            inputs.disconnect();
            assert!(inputs.is_disconnected());
            assert_eq!(outputs.send(8).await, Err(SendError::Disconnected));
            let mut receiving = Box::pin(inputs.recv());
            assert_pending(receiving.as_mut()).await;
            if deliver {
                permit.send(PortEvent::Message(7));
                assert_eq!(receiving.await.unwrap(), Some(7));
            } else {
                drop(permit);
                assert_eq!(receiving.await.unwrap(), None);
            }
            assert_eq!(inputs.recv_event().await.unwrap(), None);
            assert_eq!(Port::capacity(&inputs), Some(1));
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_terminal_marker_does_not_wait_for_or_accept_reserved_tail_events() {
    bounded(async {
        let drops = Arc::new(AtomicUsize::new(0));
        let (outputs, mut inputs) = Channel::<Tracked>::bounded(2).into_inner();
        let permit = outputs.as_ref().clone().reserve_owned().await.unwrap();
        outputs.send_event(PortEvent::Disconnect).await.unwrap();
        assert!(matches!(
            inputs.recv_event().await.unwrap(),
            Some(PortEvent::Disconnect)
        ));
        assert!(inputs.recv().await.unwrap().is_none());
        let sender = permit.send(PortEvent::Message(tracked(1, &drops)));
        assert!(inputs.recv_event().await.unwrap().is_none());
        assert!(sender.is_closed());
        drop(sender);
        drop(outputs);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_raw_sender_keeps_the_connection_alive_after_output_close() {
    bounded(async {
        let (mut outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
        let sender = outputs.as_ref().clone();
        outputs.close();
        assert!(inputs.is_connected());
        let mut receiving = Box::pin(inputs.recv());
        assert_pending(receiving.as_mut()).await;
        sender.send(PortEvent::Message(1)).await.unwrap();
        assert_eq!(receiving.await.unwrap(), Some(1));
        drop(sender);
        assert!(inputs.is_disconnected());
        assert_eq!(inputs.recv_event().await.unwrap(), None);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn closing_the_last_output_wakes_an_empty_receive() {
    bounded(async {
        let (mut outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
        let mut receiving = Box::pin(inputs.recv_event());
        assert_pending(receiving.as_mut()).await;
        outputs.close();
        assert_eq!(receiving.await.unwrap(), None); // No synthetic Disconnect.
        assert!(inputs.is_disconnected());
        assert_eq!(inputs.recv().await.unwrap(), None);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_a_send_drops_only_its_payload_and_releases_capacity() {
    bounded(async {
        for release_capacity in [false, true] {
            let drops = Arc::new(AtomicUsize::new(0));
            let (outputs, mut inputs) = Channel::<Tracked>::bounded(1).into_inner();
            outputs.send(tracked(1, &drops)).await.unwrap();
            let mut sending = Box::pin(outputs.send(tracked(2, &drops)));
            assert_pending(sending.as_mut()).await;
            if release_capacity {
                let message = inputs.recv().await.unwrap().unwrap();
                assert_eq!(message.value, 1);
                drop(message);
            }
            drop(sending); // Also exercises cancellation after a capacity wakeup.
            assert_eq!(
                drops.load(Ordering::SeqCst),
                if release_capacity { 2 } else { 1 }
            );
            if !release_capacity {
                let message = inputs.recv().await.unwrap().unwrap();
                assert_eq!(message.value, 1);
                drop(message);
            }
            assert_eq!(Port::capacity(&outputs), Some(1));
            outputs.send(tracked(3, &drops)).await.unwrap();
            let message = inputs.recv().await.unwrap().unwrap();
            assert_eq!(message.value, 3);
            drop(message);
            assert!(inputs.is_empty());
            assert_eq!(drops.load(Ordering::SeqCst), 3);
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_a_receive_after_a_wakeup_preserves_the_next_event() {
    bounded(async {
        for receive_events in [false, true] {
            for event in [
                PortEvent::Message(7),
                PortEvent::Connect,
                PortEvent::Disconnect,
            ] {
                let (outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
                let mut receiving = Box::pin(async {
                    if receive_events {
                        inputs.recv_event().await
                    } else {
                        inputs
                            .recv()
                            .await
                            .map(|message| message.map(PortEvent::Message))
                    }
                });
                assert_pending(receiving.as_mut()).await;
                outputs.send_event(event).await.unwrap();
                drop(receiving);
                assert!(inputs.is_connected());
                assert_eq!(inputs.recv_event().await.unwrap(), Some(event));
                assert!(inputs.is_empty());
            }
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_a_message_receive_may_filter_connect_but_preserves_payloads() {
    bounded(async {
        let (outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
        outputs.send_event(PortEvent::Connect).await.unwrap();
        let mut receiving = Box::pin(inputs.recv());
        assert_pending(receiving.as_mut()).await;
        drop(receiving);
        assert!(inputs.is_empty());
        assert!(inputs.is_connected());
        outputs.send(7).await.unwrap();
        assert_eq!(inputs.recv().await.unwrap(), Some(7));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_recv_all_drops_accumulated_messages() {
    bounded(async {
        let drops = Arc::new(AtomicUsize::new(0));
        let (outputs, mut inputs) = Channel::<Tracked>::bounded(2).into_inner();
        outputs.send(tracked(1, &drops)).await.unwrap();
        outputs.send(tracked(2, &drops)).await.unwrap();
        let mut collecting = InputPort::recv_all(&mut inputs);
        assert_pending(collecting.as_mut()).await;
        drop(collecting);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert!(inputs.is_empty());
        assert!(inputs.is_connected());
        outputs.send(tracked(3, &drops)).await.unwrap();
        let message = inputs.recv().await.unwrap().unwrap();
        assert_eq!(message.value, 3);
        drop(message);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn already_closed_raw_transports_preserve_buffered_input_and_report_disconnection() {
    bounded(async {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send(PortEvent::Message(7u8)).await.unwrap();
        receiver.close();
        let mut inputs = Inputs::<u8>::from(receiver);
        let outputs = Outputs::<u8>::from(sender);
        assert!(inputs.is_disconnected());
        assert!(outputs.is_disconnected());
        assert_eq!(Port::capacity(&outputs), None);
        assert_eq!(outputs.send(8).await, Err(SendError::Disconnected));
        assert_eq!(inputs.recv().await.unwrap(), Some(7));
        assert_eq!(inputs.recv().await.unwrap(), None);
        assert!(inputs.as_ref().is_closed());
        assert_eq!(inputs.recv_event().await.unwrap(), None);
    })
    .await;
}

#[cfg_attr(
    feature = "parallel",
    tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(not(feature = "parallel"), tokio::test(flavor = "current_thread"))]
async fn cloned_producers_share_a_queue_without_loss_or_reordering_each_producer() {
    bounded(async {
        let (outputs, mut inputs) = Channel::<u8>::bounded(1).into_inner();
        let peer = outputs.clone();
        let first = tokio::spawn(async move {
            for message in 0..8 {
                outputs.send(message).await.unwrap();
            }
        });
        let second = tokio::spawn(async move {
            for message in 100..108 {
                peer.send(message).await.unwrap();
            }
        });
        let messages = inputs.recv_all().await.unwrap();
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(messages.len(), 16);
        assert_eq!(
            messages
                .iter()
                .copied()
                .filter(|&n| n < 100)
                .collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );
        assert_eq!(
            messages
                .iter()
                .copied()
                .filter(|&n| n >= 100)
                .collect::<Vec<_>>(),
            (100..108).collect::<Vec<_>>()
        );
        assert!(inputs.is_disconnected());
        assert_eq!(inputs.recv().await.unwrap(), None);
    })
    .await;
}
