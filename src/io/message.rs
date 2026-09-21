// This is free and unencumbered software released into the public domain.

/// The default payload type used by system preparation, with `alloc` enabled.
///
/// A message is any payload exchanged through ports. Generic port APIs can carry
/// other Rust types; their signatures specify the required trait bounds.
/// Erased values in `Other` must be `Send + Sync` so this default payload can
/// travel between block processes scheduled on Tokio runtime threads.
#[cfg(feature = "alloc")]
pub type Message = valuand::Value<alloc::boxed::Box<dyn core::any::Any + Send + Sync>>;

/// The default payload type used by system preparation, with `alloc` disabled.
///
/// A message is any payload exchanged through ports. Generic port APIs can carry
/// other Rust types; their signatures specify the required trait bounds.
#[cfg(not(feature = "alloc"))]
pub type Message = valuand::Value;

/// The value-type descriptor associated with [`Message`], with `alloc` enabled.
#[cfg(feature = "alloc")]
pub type MessageType = valuand::AnyValueType;

/// The value-type descriptor associated with [`Message`], with `alloc` disabled.
#[cfg(not(feature = "alloc"))]
pub type MessageType = valuand::ValueType;
