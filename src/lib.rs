// This is free and unencumbered software released into the public domain.

//! This crate provides async primitives for flow-based programming (FBP).
//!
//! # Features
//!
//! The default features are `all,std`. The `all` feature selects
//! `parallel,serial,stream,tokio`; it does not enable every optional integration.
//!
//! With default features disabled, backend-neutral traits, errors, and graph
//! definitions remain available. The crate always uses `alloc`; the `alloc`
//! feature selects the richer message aliases rather than enabling allocation.
//!
//! - `tokio` enables runtime ports, channels, systems, and graph preparation.
//! - `std` enables standard-library integration and implies `alloc`. Tokio stdio
//!   helpers require both `std` and `tokio`.
//! - `serial` and `parallel` enable their Tokio schedulers when both `std` and
//!   `tokio` are enabled. These flags do not enable the backend themselves.
//! - `serde` enables serialization derives and JSON error conversion.
//! - `sqlx` enables SQLx error conversion.
//! - `stream` enables the Tokio Stream dependency; stream adapters are unfinished.
//! - `flume` selects an unfinished optional backend.
//!
//! Backend types live in their backend modules. They are also reexported at the
//! crate root when exactly one backend is enabled.
//!
//! The `channel` and `fib` examples require `tokio`; `echo_lines` and `sqrt`
//! additionally require `std`. Cargo skips examples whose required features are
//! disabled when building or testing all examples.

#![no_std]
#![forbid(unsafe_code)]
//#![allow(unused)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod error;
pub use error::*;

mod io;
pub use io::*;

pub mod model;

#[cfg(feature = "flume")]
pub mod flume;
#[cfg(all(feature = "flume", not(feature = "tokio")))]
pub use flume::*;

#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(all(feature = "tokio", not(feature = "flume")))]
pub use tokio::*;

// README examples use Tokio's crate-root aliases and stdio helpers.
#[doc = include_str!("../README.md")]
#[cfg(all(doctest, feature = "tokio", feature = "std", not(feature = "flume")))]
pub struct ReadmeDoctests;
