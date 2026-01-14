// This is free and unencumbered software released into the public domain.

use core::fmt::Debug;
use dogma::{MaybeLabeled, MaybeNamed};

/// The common interface for ports, whether for input or output.
pub trait Port<T>: Debug + MaybeNamed + MaybeLabeled {}
