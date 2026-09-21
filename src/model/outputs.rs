// This is free and unencumbered software released into the public domain.

use super::{OutputPortId, PortExport, PortId, PortRegistration};
use crate::Cardinality;
use core::{
    any::{TypeId, type_name},
    marker::PhantomData,
    ops::Bound,
};

/// An output-port descriptor with a declared maximum of one message of type `T`.
///
/// Note that `Output` doesn't implement `Copy`, whereas `Input` does.
pub type Output<T> = Outputs<T, 1, 0>;

/// An output-port descriptor for messages of type `T` in a system definition.
///
/// This identifies a connection point and declares its message cardinality.
/// Runtime backends provide the sending endpoint separately.
/// Invalid const bounds are rejected when constructing or inspecting the descriptor.
///
/// ```compile_fail
/// use async_flow::model::Outputs;
/// let _ = Outputs::<u8, -2>::default();
/// ```
///
/// Note that `Outputs` doesn't implement `Copy`, whereas `Inputs` does.
///
/// # Panics
///
/// Creating a fresh descriptor panics if the library's shared output-ID sequence is
/// exhausted. Explicitly constructed or deserialized IDs do not reserve entries
/// in that sequence.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Outputs<T, const MAX: isize = -1, const MIN: isize = 0>(OutputPortId, PhantomData<T>);

impl<T: 'static, const MAX: isize, const MIN: isize> Outputs<T, MAX, MIN> {
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const MAX: isize, const MIN: isize> Default for Outputs<T, MAX, MIN> {
    fn default() -> Self {
        let _ = Self::message_cardinality();
        Self(OutputPortId::next(), PhantomData)
    }
}

impl<T, const MAX: isize, const MIN: isize> core::fmt::Debug for Outputs<T, MAX, MIN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple(&alloc::format!("Outputs<{}>", type_name::<T>()))
            .field(&self.0)
            .finish()
    }
}

impl<T, const MAX: isize, const MIN: isize> Outputs<T, MAX, MIN> {
    pub fn id(&self) -> OutputPortId {
        self.0
    }

    /// Returns the declared lower and upper bounds on messages for this port.
    ///
    /// These bounds describe message cardinality, not the number of connections.
    pub fn cardinality() -> (Bound<usize>, Bound<usize>) {
        Self::message_cardinality().bounds()
    }

    /// Returns validated message-count constraints.
    ///
    /// Invalid const-generic bounds are rejected at compile time when used.
    pub const fn message_cardinality() -> Cardinality {
        const { Cardinality::from_limits(MAX, MIN) }
    }
}

impl<T, const MAX: isize, const MIN: isize> From<&Outputs<T, MAX, MIN>>
    for PortRegistration<OutputPortId>
{
    fn from(port: &Outputs<T, MAX, MIN>) -> Self {
        Self {
            id: port.id(),
            cardinality: Some(Outputs::<T, MAX, MIN>::message_cardinality()),
        }
    }
}

impl<T, const MAX: isize, const MIN: isize> From<&Outputs<T, MAX, MIN>>
    for PortRegistration<PortId>
{
    fn from(port: &Outputs<T, MAX, MIN>) -> Self {
        Self {
            id: port.id().into(),
            cardinality: Some(Outputs::<T, MAX, MIN>::message_cardinality()),
        }
    }
}

impl<T: Send + 'static, const MAX: isize, const MIN: isize> From<&Outputs<T, MAX, MIN>>
    for PortExport<OutputPortId>
{
    fn from(port: &Outputs<T, MAX, MIN>) -> Self {
        Self {
            id: port.id(),
            type_id: port.type_id(),
            cardinality: Some(Outputs::<T, MAX, MIN>::message_cardinality()),
            #[cfg(feature = "tokio")]
            channel_factory: Some(crate::tokio::ChannelFactory::of::<T>()),
        }
    }
}

impl<T: Send + 'static, const MAX: isize, const MIN: isize> From<&Outputs<T, MAX, MIN>>
    for PortExport<PortId>
{
    fn from(port: &Outputs<T, MAX, MIN>) -> Self {
        Self {
            id: port.id().into(),
            type_id: port.type_id(),
            cardinality: Some(Outputs::<T, MAX, MIN>::message_cardinality()),
            #[cfg(feature = "tokio")]
            channel_factory: Some(crate::tokio::ChannelFactory::of::<T>()),
        }
    }
}

impl<T, const MAX: isize, const MIN: isize> Into<OutputPortId> for &Outputs<T, MAX, MIN> {
    fn into(self) -> OutputPortId {
        self.0
    }
}

impl<T: 'static, const MAX: isize, const MIN: isize> Into<(OutputPortId, TypeId)>
    for &Outputs<T, MAX, MIN>
{
    fn into(self) -> (OutputPortId, TypeId) {
        (self.0, self.type_id())
    }
}

impl<T, const MAX: isize, const MIN: isize> Into<PortId> for &Outputs<T, MAX, MIN> {
    fn into(self) -> PortId {
        self.0.into()
    }
}

impl<T: 'static, const MAX: isize, const MIN: isize> Into<(PortId, TypeId)>
    for &Outputs<T, MAX, MIN>
{
    fn into(self) -> (PortId, TypeId) {
        (self.0.into(), self.type_id())
    }
}
