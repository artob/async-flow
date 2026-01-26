// This is free and unencumbered software released into the public domain.

use crate::Scheduler;
use tokio::runtime::{Builder, Runtime};

#[derive(Debug)]
pub struct ParallelScheduler {
    runtime: Runtime,
}

impl Scheduler for ParallelScheduler {}

impl ParallelScheduler {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        Ok(Self { runtime })
    }

    #[cfg(feature = "tokio")]
    pub fn id(&self) -> tokio::runtime::Id {
        self.runtime.handle().id()
    }
}

#[cfg(feature = "tokio")]
impl AsRef<tokio::runtime::Runtime> for ParallelScheduler {
    fn as_ref(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }
}

#[cfg(feature = "tokio")]
impl AsRef<tokio::runtime::Handle> for ParallelScheduler {
    fn as_ref(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }
}
