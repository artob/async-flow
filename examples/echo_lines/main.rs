// This is free and unencumbered software released into the public domain.

use async_flow::{Result, System};

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result {
    System::run(|s| {
        let stdin = s.read_stdin::<String>();
        let stdout = s.write_stdout::<String>();
        s.connect(stdin, stdout);
    })
    .await
}
