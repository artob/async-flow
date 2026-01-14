// This is free and unencumbered software released into the public domain.

use crate::io::Result;
use alloc::string::String;
use core::str::FromStr;

#[cfg(feature = "tokio")]
pub async fn stdin<T: FromStr>(output: crate::tokio::Output<T>) -> Result {
    use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

    let input = tokio::io::stdin();
    let reader = BufReader::new(input);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if let Ok(value) = line.parse() {
            output.send(value).await?;
        }
    }

    Ok(())
}
