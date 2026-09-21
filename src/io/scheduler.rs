// This is free and unencumbered software released into the public domain.

/// A scheduler for block processes.
///
/// Backends determine how executions of blocks are driven. This is currently a
/// marker trait; concrete schedulers provide their own scheduling APIs.
#[async_trait::async_trait]
pub trait Scheduler {}
