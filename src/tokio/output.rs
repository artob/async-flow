// This is free and unencumbered software released into the public domain.

use super::Outputs;

/// An output whose sender clones share a maximum of one message payload.
pub type Output<T> = Outputs<T, 1>;
