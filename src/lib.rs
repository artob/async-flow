// This is free and unencumbered software released into the public domain.

//! This crate provides async flow traits.

#![no_std]
#![forbid(unsafe_code)]
#![allow(unused)]

extern crate alloc;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

mod bounded;
pub use bounded::*;

mod input_port;
pub use input_port::*;

mod output_port;
pub use output_port::*;

mod port;
pub use port::*;
