// This is free and unencumbered software released into the public domain.

use async_flow::{Inputs, Outputs, Result, System};

/// cargo run --example sqrt
#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result {
    System::run(|s| {
        let stdin = s.read_stdin::<f64>();
        let stdout = s.write_stdout::<f64>();
        s.spawn(sqrt(stdin, stdout));
    })
    .await
}

/// A block that computes the square root of input numbers.
async fn sqrt(mut inputs: Inputs<f64>, outputs: Outputs<f64>) -> Result {
    while let Some(input) = inputs.recv().await? {
        let output = input.sqrt();
        outputs.send(output).await?;
    }
    Ok(())
}
