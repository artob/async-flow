// This is free and unencumbered software released into the public domain.

use async_flow::{Channel, Outputs, Result};

/// cargo run --example fib
#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let mut fibs = Channel::<u64>::bounded(1);

    tokio::spawn(fib(fibs.tx));

    while let Some(n) = fibs.rx.recv().await.unwrap() {
        println!("{}", n);
    }
}

/// A block that generates the 64-bit Fibonacci sequence.
async fn fib(outputs: Outputs<u64>) -> Result {
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    while let Some(c) = a.checked_add(b) {
        outputs.send(b).await?;
        (a, b) = (b, c);
    }
    outputs.send(b).await?;
    Ok(())
}
