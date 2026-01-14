// This is free and unencumbered software released into the public domain.

use crate::io::Result;
use alloc::string::{String, ToString};

#[cfg(feature = "tokio")]
pub async fn stdout<T: ToString>(mut input: crate::tokio::Input<T>) -> Result {
    while let Some(value) = input.recv().await? {
        std::println!("{}", value.to_string());
    }

    Ok(())
}
