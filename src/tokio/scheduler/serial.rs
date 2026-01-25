// This is free and unencumbered software released into the public domain.

use tokio::{
    runtime::{Builder, Runtime},
    task::LocalSet,
};

#[derive(Debug)]
pub struct SerialScheduler {
    runtime: Runtime,
    local_set: LocalSet,
}

impl SerialScheduler {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let local_set = LocalSet::new();
        Ok(Self { runtime, local_set })
    }

    #[cfg(feature = "tokio")]
    pub fn id(&self) -> tokio::runtime::Id {
        self.local_set.id()
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let _ = self.local_set.spawn_local(future);
    }
}

#[cfg(feature = "tokio")]
impl AsRef<tokio::runtime::Runtime> for SerialScheduler {
    fn as_ref(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }
}

#[cfg(feature = "tokio")]
impl AsRef<tokio::runtime::Handle> for SerialScheduler {
    fn as_ref(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }
}

#[cfg(feature = "tokio")]
impl AsRef<tokio::task::LocalSet> for SerialScheduler {
    fn as_ref(&self) -> &tokio::task::LocalSet {
        &self.local_set
    }
}
