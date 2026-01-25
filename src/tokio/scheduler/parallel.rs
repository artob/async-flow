// This is free and unencumbered software released into the public domain.

use tokio::runtime::{Builder, Handle, Runtime};

#[derive(Debug)]
pub struct ParallelScheduler {
    runtime: Runtime,
}

impl ParallelScheduler {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        Ok(Self { runtime })
    }
}

impl AsRef<Runtime> for ParallelScheduler {
    fn as_ref(&self) -> &Runtime {
        &self.runtime
    }
}

impl AsRef<Handle> for ParallelScheduler {
    fn as_ref(&self) -> &Handle {
        self.runtime.handle()
    }
}
