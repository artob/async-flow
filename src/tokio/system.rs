// This is free and unencumbered software released into the public domain.

use crate::{
    io::Result,
    tokio::{Input, Output},
};
use alloc::{string::ToString, vec::Vec};
use core::str::FromStr;
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

    pub fn stdin<T: FromStr>(&mut self) -> Input<T>
    where
        T: Send + 'static,
        <T as FromStr>::Err: Send,
    {
        let (output, input) = super::bounded(1);
        let block = crate::stdio::stdin(output);
        self.blocks.spawn(block);
        input
    }

    pub fn stdout<T: ToString>(&mut self) -> Output<T>
    where
        T: Send + 'static,
    {
        let (output, input) = super::bounded(1);
        let block = crate::stdio::stdout(input);
        self.blocks.spawn(block);
        output
    }
}
