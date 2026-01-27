// This is free and unencumbered software released into the public domain.

use core::{
    any::type_name,
    marker::PhantomData,
    ops::Bound,
    sync::atomic::{AtomicIsize, Ordering},
};

pub type OutputId = isize;

/// A one-shot output port of type `T`.
///
/// Note that `Output` doesn't implement `Copy`, whereas `Input` does.
pub type Output<T> = Outputs<T, 1, 0>;

/// An output port of type `T`.
///
/// Note that `Outputs` doesn't implement `Copy`, whereas `Inputs` does.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Outputs<T, const MAX: isize = -1, const MIN: isize = 0>(OutputId, PhantomData<T>);

impl<T, const MAX: isize, const MIN: isize> Default for Outputs<T, MAX, MIN> {
    fn default() -> Self {
        static COUNTER: AtomicIsize = AtomicIsize::new(1);
        let id = COUNTER.fetch_add(1, Ordering::AcqRel);
        Self(id, PhantomData)
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
    pub fn id(&self) -> OutputId {
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

impl<T, const MAX: isize, const MIN: isize> Into<OutputId> for &Outputs<T, MAX, MIN> {
    fn into(self) -> OutputId {
        self.0
    }
}
