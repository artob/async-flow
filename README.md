# Async-Flow

[![License](https://img.shields.io/badge/license-Public%20Domain-blue.svg)](https://unlicense.org)
[![Compatibility](https://img.shields.io/badge/rust-1.85%2B-blue)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
[![Package](https://img.shields.io/crates/v/async-flow)](https://crates.io/crates/async-flow)
[![Documentation](https://docs.rs/async-flow/badge.svg)](https://docs.rs/async-flow)

_"Τὰ πάντα ῥεῖ καὶ οὐδὲν μένει" — Heraclitus_

**Async abstractions for [flow-based programming] (FBP) in Rust.**
This crate can be used to implement dataflow systems consisting of
interconnected blocks that process arbitrary messages.

> [!TIP]
> 🚧 _We are building in public. This is presently under heavy construction._

<sub>

[[Features](#-features)] |
[[Prerequisites](#%EF%B8%8F-prerequisites)] |
[[Installation](#%EF%B8%8F-installation)] |
[[Examples](#-examples)] |
[[Reference](#-reference)] |
[[Development](#%E2%80%8D-development)]

</sub>

## ✨ Features

- Provides primitives for flow-based programming (FBP) based on [Tokio].
- Constructs dataflow systems by connecting reusable components called blocks.
- Compatible with the inventory of blocks provided by the [Flows.rs] project.
- Supports opting out of any feature using comprehensive feature flags.
- Adheres to the Rust API Guidelines in its [naming conventions].
- No licensing headaches: 100% free and unencumbered public domain software.

## 🛠️ Prerequisites

- [Rust](https://rust-lang.org) 1.85+ (2024 edition)

## ⬇️ Installation

### Installation via Cargo

```bash
cargo add async-flow
```

## 👉 Examples

### Importing the Library

```rust
use async_flow::*;
```

### Composing Systems

#### Reading from stdin and writing to stdout

```rust
use async_flow::{Inputs, Outputs, Result, System};

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result {
    System::run(|s| {
        let stdin = s.read_stdin::<f64>();
        let stdout = s.write_stdout::<f64>();
        s.spawn(sqrt(stdin, stdout));
    })
    .await
}

/// A block that computes the square root of input numbers.
async fn sqrt(mut inputs: Inputs<f64>, outputs: Outputs<f64>) -> Result {
    while let Some(input) = inputs.recv().await? {
        let output = input.sqrt();
        outputs.send(output).await?;
    }
    Ok(())
}
```

### Implementing Blocks

#### Implementing a `split_string` block

```rust
use async_flow::{Inputs, Outputs, Result};

/// A block that splits input strings based on a delimiter.
async fn split_string(delim: &str, mut inputs: Inputs<String>, outputs: Outputs<String>) -> Result {
    while let Some(input) = inputs.recv().await? {
        for output in input.split(delim) {
            outputs.send(output.into()).await?;
        }
    }
    Ok(())
}
```

#### Implementing an `add_ints` block

```rust
use async_flow::{Inputs, Outputs, Result};

/// A block that outputs the sums of input numbers.
async fn add_ints(mut lhs: Inputs<i64>, mut rhs: Inputs<i64>, sums: Outputs<i64>) -> Result {
    loop {
        let (a, b) = tokio::try_join!(lhs.recv(), rhs.recv())?;
        match (a, b) {
            (Some(a), Some(b)) => sums.send(a + b).await?,
            _ => break,
        }
    }
    Ok(())
}
```

## 📚 Reference

[docs.rs/async-flow](https://docs.rs/async-flow)

### Glossary

- **System**: A collection of blocks that are connected together.
  Systems are the top-level entities in dataflow programs.

- **Block**: An encapsulated system component that processes messages.
  Blocks are the autonomous units of computation in a system.

- **Port**: A named connection point on a block that sends or receives
  messages. Ports are the only interfaces through which blocks communicate
  with each other.

- **Message**: A unit of data that flows between blocks in a system, from port
  to port. Any Rust type that implements the `Send + Sync + 'static` traits can
  be used as a message.

## 👨‍💻 Development

```bash
git clone https://github.com/artob/async-flow.git
```

---

[![Share on X](https://img.shields.io/badge/share%20on-x-03A9F4?logo=x)](https://x.com/intent/post?url=https://github.com/artob/async-flow&text=Async-Flow)
[![Share on Reddit](https://img.shields.io/badge/share%20on-reddit-red?logo=reddit)](https://reddit.com/submit?url=https://github.com/artob/async-flow&title=Async-Flow)
[![Share on Hacker News](https://img.shields.io/badge/share%20on-hn-orange?logo=ycombinator)](https://news.ycombinator.com/submitlink?u=https://github.com/artob/async-flow&t=Async-Flow)
[![Share on Facebook](https://img.shields.io/badge/share%20on-fb-1976D2?logo=facebook)](https://www.facebook.com/sharer/sharer.php?u=https://github.com/artob/async-flow)
[![Share on LinkedIn](https://img.shields.io/badge/share%20on-linkedin-3949AB?logo=linkedin)](https://www.linkedin.com/sharing/share-offsite/?url=https://github.com/artob/async-flow)

[Flows.rs]: https://github.com/artob/flows.rs
[Tokio]: https://tokio.rs
[flow-based programming]: https://jpaulm.github.io/fbp/
[naming conventions]: https://rust-lang.github.io/api-guidelines/naming.html
