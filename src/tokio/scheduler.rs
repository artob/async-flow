// This is free and unencumbered software released into the public domain.

#[cfg(feature = "parallel")]
mod parallel;
#[cfg(feature = "parallel")]
pub use parallel::*;

#[cfg(feature = "serial")]
mod serial;
#[cfg(feature = "serial")]
pub use serial::*;
