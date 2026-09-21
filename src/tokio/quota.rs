// This is free and unencumbered software released into the public domain.

use crate::{Cardinality, SendError};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

/// Shared by both endpoints and every cloned output of a guarded connection.
pub(crate) struct Quota {
    pub(crate) bounds: Cardinality,
    sent: AtomicUsize,
    pub(crate) exhausted: Notify,
}

impl Quota {
    pub(crate) fn new(bounds: Cardinality) -> Arc<Self> {
        Arc::new(Self {
            bounds,
            sent: AtomicUsize::new(0),
            exhausted: Notify::new(),
        })
    }

    pub(crate) fn check(&self) -> Result<(), SendError> {
        if let Some(maximum) = self.bounds.max()
            && self.sent.load(Ordering::Acquire) >= maximum
        {
            return Err(SendError::CardinalityExceeded { maximum });
        }
        Ok(())
    }

    // Called only with a transport permit in hand, immediately before sending.
    // No await separates committing this count from enqueueing the payload.
    pub(crate) fn commit(&self) -> Result<(), SendError> {
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
            self.exhausted.notify_waiters();
        }
    }

    pub(crate) async fn wait_exhausted(&self) -> SendError {
        loop {
            let notified = self.exhausted.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Err(error) = self.check() {
                return error;
            }
            notified.await;
        }
    }
}
