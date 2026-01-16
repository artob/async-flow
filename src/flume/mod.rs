// This is free and unencumbered software released into the public domain.

pub fn bounded<T>(buffer: usize) -> (Outputs<T>, Inputs<T>) {
    let (tx, rx) = flume::bounded(buffer);
    let outputs = Outputs::from(tx);
    let inputs = Inputs::from(rx);
    (outputs, inputs)
}

mod inputs;
pub use inputs::*;

mod outputs;
pub use outputs::*;
