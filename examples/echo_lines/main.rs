// This is free and unencumbered software released into the public domain.

use async_flow::{Result, System};

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result {
    System::run(|system| {
        let stdin = system.read_stdin::<String>();
        let stdout = system.write_stdout::<String>();
        system.connect(stdin, stdout);
    })
    .await
}
