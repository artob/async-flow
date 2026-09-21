// This is free and unencumbered software released into the public domain.

use crate::{Cardinality, SendError};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Mutex, MutexGuard, Notify};

/// Finite shared input budgets reserve every producer's minimum before extras.
pub(crate) struct GroupBudget {
    maximum: Option<usize>,
    extra_limit: Option<usize>,
    extras: AtomicUsize,
    gate: Mutex<()>,
    pub(crate) changed: Arc<Notify>,
}

impl GroupBudget {
    pub(crate) fn new(bounds: Cardinality, minimums: usize) -> Option<Arc<Self>> {
        let extra_limit = match bounds.max() {
            Some(maximum) => Some(maximum.checked_sub(minimums)?),
            None => None,
        };
        Some(Arc::new(Self {
            maximum: bounds.max(),
            extra_limit,
            extras: AtomicUsize::new(0),
            gate: Mutex::new(()),
            changed: Arc::new(Notify::new()),
        }))
    }

    fn check_extra(&self) -> Result<(), SendError> {
        if let Some(limit) = self.extra_limit
            && self.extras.load(Ordering::Acquire) >= limit
        {
            return Err(SendError::FanInBudgetExhausted {
                maximum: self.maximum.expect("finite extra budget"),
            });
        }
        Ok(())
    }
}

/// Shared by both endpoints and every cloned output of a guarded connection.
pub(crate) struct Quota {
    pub(crate) bounds: Cardinality,
    sent: AtomicUsize,
    pub(crate) exhausted: Notify,
    group: Option<Arc<GroupBudget>>,
}

impl Quota {
    pub(crate) fn new(bounds: Cardinality) -> Arc<Self> {
        Arc::new(Self {
            bounds,
            sent: AtomicUsize::new(0),
            exhausted: Notify::new(),
            group: None,
        })
    }

    pub(crate) fn grouped(bounds: Cardinality, group: Arc<GroupBudget>) -> Arc<Self> {
        Arc::new(Self {
            bounds,
            sent: AtomicUsize::new(0),
            exhausted: Notify::new(),
            group: Some(group),
        })
    }

    pub(crate) fn is_grouped(&self) -> bool {
        self.group.is_some()
    }

    pub(crate) async fn lock_group(&self) -> Option<MutexGuard<'_, ()>> {
        match &self.group {
            Some(group) => Some(group.gate.lock().await),
            None => None,
        }
    }

    fn notifier(&self) -> &Notify {
        self.group
            .as_ref()
            .map_or(&self.exhausted, |group| &group.changed)
    }

    pub(crate) fn check(&self) -> Result<(), SendError> {
        if let Some(maximum) = self.bounds.max()
            && self.sent.load(Ordering::Acquire) >= maximum
        {
            return Err(SendError::CardinalityExceeded { maximum });
        }
        if let Some(group) = &self.group
            && self.sent.load(Ordering::Acquire) >= self.bounds.min()
        {
            group.check_extra()?;
        }
        Ok(())
    }

    // Called only with a transport permit in hand, immediately before sending.
    // No await separates committing this count from enqueueing the payload.
    pub(crate) fn commit(&self) -> Result<(), SendError> {
        if let Some(group) = &self.group {
            // Outputs holds the group's gate through this update and enqueue.
            self.check()?;
            let sent = self.sent.load(Ordering::Acquire);
            if sent >= self.bounds.min() && group.extra_limit.is_some() {
                group.extras.fetch_add(1, Ordering::AcqRel);
            }
            if self.bounds.max().is_some() || sent < self.bounds.min() {
                self.sent.store(sent + 1, Ordering::Release);
            }
            return Ok(());
        }
        if let Some(maximum) = self.bounds.max() {
            self.sent
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |sent| {
                    if sent < maximum { Some(sent + 1) } else { None }
                })
                .map_err(|_| SendError::CardinalityExceeded { maximum })?;
        }
        Ok(())
    }

    pub(crate) fn notify_if_exhausted(&self) {
        if self.check().is_err() {
            self.notifier().notify_waiters();
        }
    }

    pub(crate) async fn wait_exhausted(&self) -> SendError {
        loop {
            let notified = self.notifier().notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Err(error) = self.check() {
                return error;
            }
            notified.await;
        }
    }
}
