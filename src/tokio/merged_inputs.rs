// This is free and unencumbered software released into the public domain.

use super::{
    Inputs, Outputs, SystemPrepareError,
    channel_factory::FanInConnection,
    quota::{GroupBudget, Quota},
};
use crate::{Cardinality, PortEvent, PortState, RecvError, model::OutputPortId};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::{Notify, mpsc};

/// Opaque storage for a receiving port that fairly merges producer connections.
///
/// Each producer has its own bounded queue and FIFO order. There is no total
/// cross-producer enqueue order. Source-local disconnect markers are consumed
/// by the merger; they close only that source and discard its trailing events.
/// The merged port reports EOF when all sources finish or its maximum is read.
/// Any source cardinality error terminates the merge and is propagated once.
pub struct MergedInputs<T> {
    sources: Vec<(OutputPortId, Inputs<T>)>,
    done: Vec<bool>,
    next: usize,
    changed: Arc<Notify>,
    wake: Pin<Box<tokio::sync::futures::OwnedNotified>>,
}

impl<T> MergedInputs<T> {
    fn new(sources: Vec<(OutputPortId, Inputs<T>)>, changed: Arc<Notify>) -> Self {
        let done = alloc::vec![false; sources.len()];
        let wake = Box::pin(Arc::clone(&changed).notified_owned());
        Self {
            sources,
            done,
            next: 0,
            changed,
            wake,
        }
    }

    pub(crate) fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<PortEvent<T>>, RecvError>> {
        // Register before checking source budgets. Completion can retire a source
        // even when its native sender handles remain alive and its queue is empty.
        self.wake.as_mut().enable();
        if self.wake.as_mut().poll(cx).is_ready() {
            self.wake = Box::pin(Arc::clone(&self.changed).notified_owned());
            self.wake.as_mut().enable();
            if self.wake.as_mut().poll(cx).is_ready() {
                cx.waker().wake_by_ref();
            }
        }
        let count = self.sources.len();
        for offset in 0..count {
            let index = (self.next + offset) % count;
            if self.done[index] {
                continue;
            }
            let (output, input) = &mut self.sources[index];
            if input.quota.check().is_err() {
                input.disconnect();
            }
            match input.poll_recv_event(cx) {
                Poll::Pending => (),
                Poll::Ready(Ok(None | Some(PortEvent::Disconnect))) => {
                    self.done[index] = true;
                    input.close();
                },
                Poll::Ready(Ok(Some(event))) => {
                    self.next = (index + 1) % count;
                    return Poll::Ready(Ok(Some(event)));
                },
                Poll::Ready(Err(RecvError::CardinalityUnderflow { minimum, received })) => {
                    return Poll::Ready(Err(RecvError::ProducerCardinalityUnderflow {
                        output: *output,
                        minimum,
                        received,
                    }));
                },
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            }
        }
        if self.done.iter().all(|done| *done) {
            Poll::Ready(Ok(None))
        } else {
            Poll::Pending
        }
    }

    pub(crate) fn check_minima(&self) -> Result<(), RecvError> {
        for (output, input) in &self.sources {
            let minimum = input.cardinality().min();
            if input.received_count() < minimum {
                return Err(RecvError::ProducerCardinalityUnderflow {
                    output: *output,
                    minimum,
                    received: input.received_count(),
                });
            }
        }
        Ok(())
    }

    pub(crate) fn disconnect(&mut self) {
        for (_, input) in &mut self.sources {
            input.disconnect();
        }
    }

    pub(crate) fn state(&self) -> PortState {
        if self
            .sources
            .iter()
            .any(|(_, input)| input.state() == PortState::Connected)
        {
            PortState::Connected
        } else {
            PortState::Disconnected
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sources.iter().all(|(_, input)| input.is_empty())
    }

    pub(crate) fn capacity(&self, maximum: bool) -> Option<usize> {
        let mut total = None::<usize>;
        for (_, input) in &self.sources {
            let capacity = if maximum {
                input.max_capacity()
            } else {
                input.capacity()
            };
            if let Some(capacity) = capacity {
                total = Some(total.unwrap_or(0).saturating_add(capacity));
            }
        }
        total
    }
}

fn projected_ranges(
    total: Cardinality,
    sources: &[(OutputPortId, Cardinality)],
) -> Result<Vec<Cardinality>, SystemPrepareError> {
    let mut result = Vec::with_capacity(sources.len());
    for (index, (_, own)) in sources.iter().enumerate() {
        let mut other_min = 0usize;
        let mut other_max = Some(0usize);
        for (other_index, (_, other)) in sources.iter().enumerate() {
            if index == other_index {
                continue;
            }
            other_min = other_min
                .checked_add(other.min())
                .ok_or(SystemPrepareError::FanInCardinalityOverflow)?;
            other_max = match (other_max, other.max()) {
                (Some(a), Some(b)) => a.checked_add(b),
                _ => None,
            };
        }
        let minimum = own
            .min()
            .max(other_max.map_or(0, |max| total.min().saturating_sub(max)));
        let maximum = match (own.max(), total.max()) {
            (own, None) => own,
            (own, Some(max)) => {
                let available = max
                    .checked_sub(other_min)
                    .ok_or(SystemPrepareError::FanInCardinalityOverflow)?;
                Some(own.map_or(available, |own| own.min(available)))
            },
        };
        result.push(
            Cardinality::new(minimum, maximum)
                .ok_or(SystemPrepareError::FanInCardinalityOverflow)?,
        );
    }
    Ok(result)
}

pub(crate) fn connect<T: Send + 'static>(
    buffer: usize,
    total: Cardinality,
    sources: &[(OutputPortId, Cardinality)],
) -> Result<FanInConnection, SystemPrepareError> {
    let ranges = projected_ranges(total, sources)?;
    let minimums = ranges
        .iter()
        .try_fold(0usize, |sum, range| sum.checked_add(range.min()))
        .ok_or(SystemPrepareError::FanInCardinalityOverflow)?;
    let group =
        GroupBudget::new(total, minimums).ok_or(SystemPrepareError::FanInCardinalityOverflow)?;
    let mut inputs = Vec::with_capacity(sources.len());
    let mut outputs = Vec::with_capacity(sources.len());
    for ((id, _), bounds) in sources.iter().zip(ranges) {
        let quota = Quota::grouped(bounds, Arc::clone(&group));
        let (tx, rx) = mpsc::channel(buffer);
        let output = Outputs::<T>::with_sender(tx, Arc::clone(&quota));
        let input = Inputs::<T>::with_receiver(rx, quota);
        outputs.push((
            *id,
            bounds,
            Box::new(output) as super::runtime_ports::ErasedPort,
        ));
        inputs.push((*id, input));
    }
    let input =
        Inputs::<T>::with_merged(MergedInputs::new(inputs, Arc::clone(&group.changed)), total);
    Ok(FanInConnection {
        input: Box::new(input),
        outputs,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::{Port, SendError};
    use core::{future::poll_fn, task::Poll, time::Duration};

    fn merge(
        buffer: usize,
        total: Cardinality,
        a: Cardinality,
        b: Cardinality,
    ) -> (Outputs<u8>, Outputs<u8>, Inputs<u8>) {
        let group =
            connect::<u8>(buffer, total, &[(OutputPortId(1), a), (OutputPortId(2), b)]).unwrap();
        let mut outputs = group.outputs.into_iter();
        let a = *outputs.next().unwrap().2.downcast::<Outputs<u8>>().unwrap();
        let b = *outputs.next().unwrap().2.downcast::<Outputs<u8>>().unwrap();
        let input = *group.input.downcast::<Inputs<u8>>().unwrap();
        (a, b, input)
    }

    async fn bounded<T>(future: impl Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("merge timed out")
    }

    #[tokio::test(flavor = "current_thread")]
    async fn round_robin_preserves_fifo_and_disconnect_is_source_local() {
        bounded(async {
            let (a, b, mut input) = merge(
                4,
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
            );
            a.send(1).await.unwrap();
            a.send_event(PortEvent::Disconnect).await.unwrap();
            a.send(99).await.unwrap();
            b.send(2).await.unwrap();
            b.send(3).await.unwrap();
            assert_eq!(input.recv().await.unwrap(), Some(1));
            assert_eq!(input.recv().await.unwrap(), Some(2));
            assert_eq!(input.recv().await.unwrap(), Some(3));
            assert!(a.is_disconnected());
            assert!(b.is_connected());
            b.send(4).await.unwrap();
            assert_eq!(input.recv().await.unwrap(), Some(4));
            drop(b);
            assert_eq!(input.recv_event().await.unwrap(), None); // A handle survives.
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn aggregate_budget_reserves_each_producers_minimum() {
        bounded(async {
            let (a, b, mut input) = merge(
                3,
                Cardinality::from_limits(3, 2),
                Cardinality::from_limits(-1, 1),
                Cardinality::from_limits(-1, 1),
            );
            a.send(1).await.unwrap();
            a.send(2).await.unwrap();
            assert!(a.send(3).await.is_err()); // B's required slot cannot be stolen.
            b.send(4).await.unwrap();
            let mut seen = Vec::new();
            while let Some(n) = input.recv().await.unwrap() {
                seen.push(n);
            }
            seen.sort();
            assert_eq!(seen, alloc::vec![1, 2, 4]);
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exhausted_spare_budget_retires_live_sources_without_waiting_for_drops() {
        bounded(async {
            let (a, b, mut input) = merge(
                2,
                Cardinality::from_limits(3, 0),
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
            );
            a.send(1).await.unwrap();
            b.send(2).await.unwrap();
            b.send(3).await.unwrap();
            assert_eq!(
                a.send(4).await,
                Err(SendError::FanInBudgetExhausted { maximum: 3 })
            );
            let mut seen = Vec::new();
            while let Some(n) = input.recv().await.unwrap() {
                seen.push(n);
            }
            assert_eq!(seen.len(), 3);
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn source_shortfall_is_not_masked_by_other_producers() {
        bounded(async {
            let (a, b, mut input) = merge(
                3,
                Cardinality::from_limits(5, 2),
                Cardinality::from_limits(3, 1),
                Cardinality::from_limits(3, 1),
            );
            a.send(1).await.unwrap();
            a.send(2).await.unwrap();
            b.send_event(PortEvent::Disconnect).await.unwrap();
            assert_eq!(input.recv().await.unwrap(), Some(1));
            assert_eq!(
                input.recv().await,
                Err(RecvError::ProducerCardinalityUnderflow {
                    output: OutputPortId(2),
                    minimum: 1,
                    received: 0
                })
            );
            assert_eq!(input.recv().await.unwrap(), None);
            assert!(a.is_disconnected());
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn aggregate_shortfall_is_reported_after_optional_sources_end() {
        bounded(async {
            let (a, b, mut input) = merge(
                2,
                Cardinality::from_limits(-1, 3),
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
            );
            a.send(1).await.unwrap();
            b.send(2).await.unwrap();
            drop(a);
            drop(b);
            assert_eq!(input.recv().await.unwrap(), Some(1));
            assert_eq!(input.recv().await.unwrap(), Some(2));
            assert_eq!(
                input.recv().await,
                Err(RecvError::CardinalityUnderflow {
                    minimum: 3,
                    received: 2
                })
            );
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_and_cancellation_preserve_backpressure_and_quota() {
        bounded(async {
            for close in [false, true] {
                let (a, b, mut input) = merge(
                    1,
                    Cardinality::from_limits(4, 0),
                    Cardinality::UNLIMITED,
                    Cardinality::UNLIMITED,
                );
                a.send(1).await.unwrap();
                b.send(2).await.unwrap();
                let mut sending = Box::pin(a.send(99));
                poll_fn(|cx| {
                    assert!(sending.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
                drop(sending);
                assert_eq!(input.recv().await.unwrap(), Some(1));
                a.send(3).await.unwrap();
                let mut blocked = Box::pin(a.send(4));
                poll_fn(|cx| {
                    assert!(blocked.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
                if close {
                    input.close();
                } else {
                    input.disconnect();
                }
                assert_eq!(blocked.await, Err(SendError::Disconnected));
                if !close {
                    assert_eq!(input.recv().await.unwrap(), Some(2));
                    assert_eq!(input.recv().await.unwrap(), Some(3));
                }
                assert_eq!(input.recv().await.unwrap(), None);
            }
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pending_receive_cancellation_does_not_consume_a_payload() {
        bounded(async {
            let (a, _b, mut input) = merge(
                1,
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
                Cardinality::UNLIMITED,
            );
            let mut receiving = Box::pin(input.recv());
            poll_fn(|cx| {
                assert!(receiving.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            a.send(7).await.unwrap();
            drop(receiving);
            assert_eq!(input.recv().await.unwrap(), Some(7));
        })
        .await;
    }

    #[test]
    fn merged_and_grouped_raw_access_is_guarded_even_without_count_constraints() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let (a, _, input) = merge(
            1,
            Cardinality::UNLIMITED,
            Cardinality::UNLIMITED,
            Cardinality::UNLIMITED,
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = a.as_ref();
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = input.as_ref();
            }))
            .is_err()
        );
    }

    #[cfg(feature = "parallel")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_clones_cannot_overspend_aggregate_or_reserved_budget() {
        bounded(async {
            let (a, b, mut input) = merge(
                8,
                Cardinality::from_limits(10, 2),
                Cardinality::from_limits(-1, 1),
                Cardinality::from_limits(-1, 1),
            );
            let barrier = Arc::new(tokio::sync::Barrier::new(33));
            let mut tasks = tokio::task::JoinSet::new();
            for n in 0..32u8 {
                let sender = if n % 2 == 0 { a.clone() } else { b.clone() };
                let barrier = Arc::clone(&barrier);
                tasks.spawn(async move {
                    barrier.wait().await;
                    sender.send(n).await
                });
            }
            barrier.wait().await;
            let receiver = async {
                let mut values = Vec::new();
                while let Some(value) = input.recv().await.unwrap() {
                    values.push(value);
                }
                values
            };
            let senders = async {
                let mut successful = 0;
                while let Some(result) = tasks.join_next().await {
                    match result.unwrap() {
                        Ok(()) => successful += 1,
                        Err(
                            crate::SendError::FanInBudgetExhausted { .. }
                            | crate::SendError::CardinalityExceeded { .. }
                            | crate::SendError::Disconnected,
                        ) => (),
                        Err(error) => panic!("{error:?}"),
                    }
                }
                successful
            };
            let (values, successful) = tokio::join!(receiver, senders);
            assert_eq!(successful, 10);
            assert_eq!(values.len(), 10);
            assert!(values.iter().any(|n| n % 2 == 0));
            assert!(values.iter().any(|n| n % 2 == 1));
        })
        .await;
    }
}
