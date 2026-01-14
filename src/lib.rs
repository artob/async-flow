// This is free and unencumbered software released into the public domain.

//! This crate provides async flow traits.

#![no_std]
#![forbid(unsafe_code)]
#![allow(unused)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub mod io;

#[cfg(feature = "std")]
pub mod stdio;

#[cfg(feature = "tokio")]
pub mod tokio;
