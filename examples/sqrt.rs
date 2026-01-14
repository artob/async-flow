// This is free and unencumbered software released into the public domain.

use async_flow::{
    io::Result,
    stdio::{stdin, stdout},
    tokio::{Input, Output},
};
use tokio::task::JoinSet;

/// cargo run --example sqrt
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (input, sqrt_in) = async_flow::tokio::bounded(1);
    let (sqrt_out, output) = async_flow::tokio::bounded(1);

    let mut blocks: JoinSet<Result> = JoinSet::new();
    blocks.spawn(stdin::<f64>(input));
    blocks.spawn(sqrt(sqrt_in, sqrt_out));
    blocks.spawn(stdout::<f64>(output));

    eprintln!("{:?}", blocks.join_all().await);
}

async fn sqrt(mut input: Input<f64>, output: Output<f64>) -> Result {
    while let Some(value) = input.recv().await? {
        output.send(value.sqrt()).await?;
    }
    Ok(())
}
