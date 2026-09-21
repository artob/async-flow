// This is free and unencumbered software released into the public domain.

use super::Inputs;

/// An input accepting zero or one message payload, then reaching EOF.
pub type Input<T> = Inputs<T, 1>;
