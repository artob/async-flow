// This is free and unencumbered software released into the public domain.

use super::{InputPortId, PortId};
use core::{
    any::{TypeId, type_name},
    marker::PhantomData,
    ops::Bound,
    sync::atomic::{AtomicIsize, Ordering},
};

/// A one-shot input port of type `T`.
///
/// Note that `Input` implements `Copy`, whereas `Output` doesn't.
pub type Input<T> = Inputs<T, 1, 0>;

/// An input port of type `T`.
///
/// Note that `Inputs` implements `Copy`, whereas `Outputs` doesn't.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Inputs<T, const MAX: isize = -1, const MIN: isize = 0>(InputPortId, PhantomData<T>);

impl<T, const MAX: isize, const MIN: isize> Default for Inputs<T, MAX, MIN> {
    fn default() -> Self {
        static COUNTER: AtomicIsize = AtomicIsize::new(-1);
        let id = COUNTER.fetch_sub(1, Ordering::AcqRel);
        Self(InputPortId(id), PhantomData)
    }
}

impl<T, const MAX: isize, const MIN: isize> core::fmt::Debug for Inputs<T, MAX, MIN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple(&alloc::format!("Inputs<{}>", type_name::<T>()))
            .field(&self.0)
            .finish()
    }
}

impl<T, const MAX: isize, const MIN: isize> Inputs<T, MAX, MIN> {
    pub fn id(&self) -> InputPortId {
        self.0
    }

    /// Returns the cardinality of this connection.
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

impl<T, const MAX: isize, const MIN: isize> Into<InputPortId> for &Inputs<T, MAX, MIN> {
    fn into(self) -> InputPortId {
        self.0
    }
}

impl<T: 'static, const MAX: isize, const MIN: isize> Into<(InputPortId, TypeId)>
    for &Inputs<T, MAX, MIN>
{
    fn into(self) -> (InputPortId, TypeId) {
        (self.0, TypeId::of::<T>())
    }
}

impl<T, const MAX: isize, const MIN: isize> Into<PortId> for &Inputs<T, MAX, MIN> {
    fn into(self) -> PortId {
        self.0.into()
    }
}

impl<T: 'static, const MAX: isize, const MIN: isize> Into<(PortId, TypeId)>
    for &Inputs<T, MAX, MIN>
{
    fn into(self) -> (PortId, TypeId) {
        (self.0.into(), TypeId::of::<T>())
    }
}
