// This is free and unencumbered software released into the public domain.

use async_flow::Channel;

/// cargo run --example channel
#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let (outputs, mut inputs) = Channel::<&str>::bounded(1).into_inner();

    tokio::spawn(async move {
        outputs.send("value1").await.unwrap();
        outputs.send("value2").await.unwrap();
    });

    while let Some(message) = inputs.recv().await.unwrap() {
        eprintln!("recv: {}", message);
    }
}
