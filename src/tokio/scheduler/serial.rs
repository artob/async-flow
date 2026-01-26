// This is free and unencumbered software released into the public domain.

use crate::{Channel, Inputs, Outputs, Scheduler};
use tokio::{
    runtime::{Builder, Runtime},
    task::LocalSet,
};

#[derive(Debug)]
pub struct SerialScheduler {
    tasks: Option<LocalSet>,
    runtime: Runtime,
}

impl Scheduler for SerialScheduler {}

impl SerialScheduler {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let tasks = Some(LocalSet::new());
        Ok(Self { tasks, runtime })
    }

    #[cfg(feature = "tokio")]
    pub fn id(&self) -> Option<tokio::runtime::Id> {
        self.tasks.as_ref().map(|tasks| tasks.id())
    }

    pub fn spawn<F>(&self, process: F)
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let _ = self.tasks.as_ref().unwrap().spawn_local(process);
    }

    pub fn create<T, F>(&self, block: F) -> (Outputs<T>, Inputs<T>)
    where
        F: AsyncFn(Inputs<T>, Outputs<T>) -> Result<(), crate::Error>,
        F: Copy + 'static,
        T: 'static,
    {
        let (input, output) = Channel::<T>::pair();
        let _ = self
            .tasks
            .as_ref()
            .unwrap()
            .spawn_local(async move { block(input.rx, output.tx).await });
        (input.tx, output.rx)
    }

    pub async fn run(&mut self) {
        match self.tasks.take() {
            None => (),
            Some(tasks) => tasks.await,
        }
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
        &self.tasks.as_ref().unwrap()
    }
}
