// This is free and unencumbered software released into the public domain.

use super::Inputs;
use crate::error::Result;
use alloc::string::ToString;

pub async fn stdout<T: ToString>(mut inputs: Inputs<T>) -> Result {
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
