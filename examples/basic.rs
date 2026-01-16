// This is free and unencumbered software released into the public domain.

/// cargo run --example basic
#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let (outputs, mut inputs) = async_flow::bounded(1);

    tokio::spawn(async move {
        outputs.send("value1").await.unwrap();
        outputs.send("value2").await.unwrap();
    });

    while let Some(message) = inputs.recv().await.unwrap() {
        eprintln!("recv: {}", message);
    }
}
