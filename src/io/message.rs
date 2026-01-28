// This is free and unencumbered software released into the public domain.

#[cfg(feature = "alloc")]
pub type Message = valuand::AnyValue;

#[cfg(not(feature = "alloc"))]
pub type Message = valuand::Value;

#[cfg(feature = "alloc")]
pub type MessageType = valuand::AnyValueType;

#[cfg(not(feature = "alloc"))]
pub type MessageType = valuand::ValueType;
