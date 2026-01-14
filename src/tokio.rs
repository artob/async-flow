// This is free and unencumbered software released into the public domain.

use alloc::boxed::Box;

pub fn bounded<T>(buffer: usize) -> (Output<T>, Input<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    let output = Output::from(tx);
    let input = Input::from(rx);
    (output, input)
}

pub fn bounded_boxed<T>(
    buffer: usize,
) -> (
    Box<dyn crate::io::OutputPort<T> + Send>,
    Box<dyn crate::io::InputPort<T> + Send>,
)
where
    T: Send + Sync + 'static,
{
    let (output, input) = bounded(buffer);
    (Box::new(output), Box::new(input))
}

mod input;
pub use input::*;

mod output;
pub use output::*;

mod system;
pub use system::*;
