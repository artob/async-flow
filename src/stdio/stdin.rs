// This is free and unencumbered software released into the public domain.

use crate::io::{Error, Result};
use alloc::string::String;
use core::str::FromStr;

#[cfg(feature = "tokio")]
pub async fn stdin<T: FromStr>(outputs: crate::tokio::Outputs<T>) -> Result {
    use std::io::ErrorKind;
    use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

    let input = tokio::io::stdin();
    let reader = BufReader::new(input);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let output = line
            .parse()
            .map_err(|_| Error::Stdio(ErrorKind::InvalidInput.into()))?;
        outputs.send(output).await?;
    }

    Ok(())
}
