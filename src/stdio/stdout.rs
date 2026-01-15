// This is free and unencumbered software released into the public domain.

use crate::io::Result;
use alloc::string::{String, ToString};

#[cfg(feature = "tokio")]
pub async fn stdout<T: ToString>(mut inputs: crate::tokio::Inputs<T>) -> Result {
    use tokio::io::AsyncWriteExt;

    let mut output = tokio::io::stdout();

    while let Some(input) = inputs.recv().await? {
        let mut line = input.to_string();
        if !line.ends_with('\n') {
            line.push('\n');
        }
        output.write_all(line.as_bytes()).await?;
        output.flush().await?;
    }

    Ok(())
}
