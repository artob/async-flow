// This is free and unencumbered software released into the public domain.

use super::{BlockPorts, PortBindingError};
use crate::{Result, model::BlockDefinition};
use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

/// An owned, sendable future implementing one execution of a block.
///
/// Preparation constructs this future without polling it. `System::execute()`
/// schedules it as a Tokio task. It must own its runtime ports and execution state.
pub type ProcessFuture = Pin<Box<dyn Future<Output = Result> + Send + 'static>>;

/// A Tokio process factory associated with a structural block definition.
///
/// Register with `SystemBuilder::register_executable`. Each preparation calls the
/// factory again with fresh ports. The definition itself need not be `Send` or
/// `Sync`; the returned future must be. Copy or clone configuration into the
/// future rather than borrowing the definition or capturing its `Rc` handle.
///
/// # Examples
///
/// ```
/// use async_flow::{Result, model::{BlockDefinition, BlockName, OutputPortId,
///     Outputs, SystemBuilder}, tokio::{BlockPorts, ExecutableBlock,
///     PortBindingError, ProcessFuture}};
/// use std::borrow::Cow;
///
/// struct Source { output: Outputs<u8, 1, 1> }
/// impl BlockName for Source {
///     fn name(&self) -> Cow<'_, str> { "source".into() }
/// }
/// impl BlockDefinition for Source {
///     fn outputs(&self) -> Vec<OutputPortId> { vec![self.output.id()] }
/// }
/// impl ExecutableBlock for Source {
///     fn create_process(&self, ports: &mut BlockPorts<'_>)
///         -> Result<ProcessFuture, PortBindingError> {
///         let output = ports.take_output(&self.output)?;
///         Ok(Box::pin(async move { output.send(7).await?; Ok(()) }))
///     }
/// }
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut builder = SystemBuilder::new();
/// let source = builder.register_executable(Source { output: Default::default() });
/// builder.export_output(&source.output)?;
/// let mut system = builder.build().prepare()?;
/// let mut output = system.take_output_receiver(&source.output)?;
/// let read = async move {
///     assert_eq!(output.recv().await?, Some(7));
///     assert_eq!(output.recv().await?, None);
///     Ok::<(), async_flow::Error>(())
/// };
/// tokio::try_join!(system.execute(), read)?;
/// # Ok(())
/// # }
/// ```
pub trait ExecutableBlock: BlockDefinition {
    /// Binds this block's ports and constructs an unpolled process future.
    ///
    /// Claim every connected or exported block port exactly once. Unconnected
    /// ports may be ignored. Clone a claimed output to share its connection.
    /// Perform asynchronous initialization inside the returned future; this
    /// method runs synchronously and must not spawn tasks itself.
    ///
    /// # Errors
    ///
    /// Return binding or configuration errors to abort preparation. No library
    /// process starts until all factories succeed. Factory panics propagate to
    /// the caller of preparation, just like panics in metadata callbacks.
    fn create_process(&self, ports: &mut BlockPorts<'_>)
    -> Result<ProcessFuture, PortBindingError>;
}
