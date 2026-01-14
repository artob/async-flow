// This is free and unencumbered software released into the public domain.

use crate::{
    io::Result,
    stdio::{stdin, stdout},
    tokio::{Input, Output},
};
use alloc::vec::Vec;
use tokio::task::{AbortHandle, JoinSet};

pub struct System {
    pub blocks: JoinSet<Result>,
}

impl System {
    pub fn new() -> Self {
        Self {
            blocks: JoinSet::new(),
        }
    }

    pub fn spawn<F>(&mut self, task: F) -> AbortHandle
    where
        F: Future<Output = Result>,
        F: Send + 'static,
    {
        self.blocks.spawn(task)
    }

    pub async fn execute(self) {
        self.blocks.join_all().await;
    }
}
