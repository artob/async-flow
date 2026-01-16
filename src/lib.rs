// This is free and unencumbered software released into the public domain.

//! This crate provides async flow traits.

#![no_std]
#![forbid(unsafe_code)]
//#![allow(unused)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod io;
pub use io::*;

#[cfg(feature = "flume")]
pub mod flume;
#[cfg(all(feature = "flume", not(feature = "tokio")))]
pub use flume::*;

#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(all(feature = "tokio", not(feature = "flume")))]
pub use tokio::*;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
