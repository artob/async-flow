// This is free and unencumbered software released into the public domain.

use tokio::{
    runtime::{Builder, Handle, Runtime},
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
}

impl AsRef<Runtime> for SerialScheduler {
    fn as_ref(&self) -> &Runtime {
        &self.runtime
    }
}

impl AsRef<Handle> for SerialScheduler {
    fn as_ref(&self) -> &Handle {
        self.runtime.handle()
    }
}

impl AsRef<LocalSet> for SerialScheduler {
    fn as_ref(&self) -> &LocalSet {
        &self.local_set
    }
}
