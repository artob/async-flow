// This is free and unencumbered software released into the public domain.

use async_flow::{Inputs, Outputs, Result, System};

/// cargo run --example sqrt
#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result {
    System::run(|system| {
        let stdin = system.stdin::<f64>();
        let stdout = system.stdout::<f64>();
        system.spawn(sqrt(stdin, stdout));
    })
    .await
}

async fn sqrt(mut inputs: Inputs<f64>, outputs: Outputs<f64>) -> Result {
    while let Some(input) = inputs.recv().await? {
        let output = input.sqrt();
        outputs.send(output).await?;
    }
    Ok(())
}
