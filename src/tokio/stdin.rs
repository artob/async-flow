// This is free and unencumbered software released into the public domain.

use super::Outputs;
use crate::io::{Error, Result};
use core::str::FromStr;

pub async fn stdin<T: FromStr>(outputs: Outputs<T>) -> Result {
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
