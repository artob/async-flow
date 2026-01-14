// This is free and unencumbered software released into the public domain.

use crate::{InputPort, OutputPort};
use alloc::boxed::Box;

pub fn bounded<T>(buffer: usize) -> (BoundedOutputPort<T>, BoundedInputPort<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    let output = BoundedOutputPort::from(tx);
    let input = BoundedInputPort::from(rx);
    (output, input)
}

pub fn bounded_boxed<T>(
    buffer: usize,
) -> (Box<dyn OutputPort<T> + Send>, Box<dyn InputPort<T> + Send>)
where
    T: Send + Sync + 'static,
{
    let (output, input) = bounded(buffer);
    (Box::new(output), Box::new(input))
}

mod input_port;
pub use input_port::*;

mod output_port;
pub use output_port::*;
