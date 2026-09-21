// This is free and unencumbered software released into the public domain.

//! Two source blocks feed a formatting block through a merged input.
//! Run with `cargo run --example defined_system`.

use async_flow::{
    Result,
    model::{
        BlockDefinition, BlockName, InputPortId, Inputs, OutputPortId, Outputs, SystemBuilder,
    },
    tokio::{BlockPorts, ExecutableBlock, PortBindingError, ProcessFuture},
};
use std::borrow::Cow;

struct Source {
    value: u64,
    output: Outputs<u64, 1, 1>,
}
impl BlockName for Source {
    fn name(&self) -> Cow<'_, str> {
        "source".into()
    }
}
impl BlockDefinition for Source {
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Source {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let output = ports.take_output(&self.output)?;
        let value = self.value;
        Ok(Box::pin(async move {
            output.send(value).await?;
            Ok(())
        }))
    }
}

struct Format {
    input: Inputs<u64, 2, 2>,
    output: Outputs<String, 2, 2>,
}
impl BlockName for Format {
    fn name(&self) -> Cow<'_, str> {
        "format".into()
    }
}
impl BlockDefinition for Format {
    fn inputs(&self) -> Vec<InputPortId> {
        vec![self.input.id()]
    }
    fn outputs(&self) -> Vec<OutputPortId> {
        vec![self.output.id()]
    }
}
impl ExecutableBlock for Format {
    fn create_process(
        &self,
        ports: &mut BlockPorts<'_>,
    ) -> Result<ProcessFuture, PortBindingError> {
        let mut input = ports.take_input(&self.input)?;
        let output = ports.take_output(&self.output)?;
        Ok(Box::pin(async move {
            while let Some(value) = input.recv().await? {
                output.send(format!("value: {value}")).await?;
            }
            Ok(())
        }))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SystemBuilder::new();
    let a = builder.register_executable(Source {
        value: 4,
        output: Default::default(),
    });
    let b = builder.register_executable(Source {
        value: 9,
        output: Default::default(),
    });
    let format = builder.register_executable(Format {
        input: Default::default(),
        output: Default::default(),
    });
    builder.connect(&a.output, &format.input)?;
    builder.connect(&b.output, &format.input)?;
    builder.export_output(&format.output)?;
    // No runtime is needed until execution. Take exported endpoints first.
    let mut system = builder.build().prepare()?;
    let mut output = system.take_output_receiver(&format.output)?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    runtime.block_on(async move {
        let print = async move {
            while let Some(line) = output.recv().await? {
                println!("{line}");
            }
            Ok::<(), async_flow::Error>(())
        };
        tokio::try_join!(system.execute(), print)?;
        Ok::<(), async_flow::Error>(())
    })?;
    Ok(())
}
