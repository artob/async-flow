// This is free and unencumbered software released into the public domain.

use super::{OutputPortId, PortId};
use core::{
    any::{TypeId, type_name},
    marker::PhantomData,
    ops::Bound,
    sync::atomic::{AtomicIsize, Ordering},
};

/// An output-port descriptor with a declared maximum of one message of type `T`.
///
/// Note that `Output` doesn't implement `Copy`, whereas `Input` does.
pub type Output<T> = Outputs<T, 1, 0>;

/// An output-port descriptor for messages of type `T` in a system definition.
///
/// This identifies a connection point and declares its message cardinality.
/// Runtime backends provide the sending endpoint separately.
///
/// Note that `Outputs` doesn't implement `Copy`, whereas `Inputs` does.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Outputs<T, const MAX: isize = -1, const MIN: isize = 0>(OutputPortId, PhantomData<T>);

impl<T: 'static, const MAX: isize, const MIN: isize> Outputs<T, MAX, MIN> {
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T, const MAX: isize, const MIN: isize> Default for Outputs<T, MAX, MIN> {
    fn default() -> Self {
        static COUNTER: AtomicIsize = AtomicIsize::new(1);
        let id = COUNTER.fetch_add(1, Ordering::AcqRel);
        Self(OutputPortId(id), PhantomData)
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
        assert!(MIN >= 0);
        assert!(MAX >= -1);
        use Bound::*;
        match (MIN, MAX) {
            (min, -1) => (Included(min as _), Unbounded),
            (min, max) => (Included(min as _), Included(max as _)),
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
