// This is free and unencumbered software released into the public domain.

use alloc::{collections::BTreeSet, vec::Vec};
use core::{fmt::Debug, ops::RangeInclusive};

/// A set of port IDs.
#[derive(Clone)]
pub struct PortIdSet<T>(BTreeSet<T>);

impl<T> Default for PortIdSet<T> {
    fn default() -> Self {
        Self(BTreeSet::default())
    }
}

impl<T: Copy + Ord> From<&Vec<T>> for PortIdSet<T> {
    fn from(input: &Vec<T>) -> Self {
        Self(BTreeSet::<T>::from_iter(input.iter().cloned()))
    }
}

impl<T: Ord> PortIdSet<T> {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn first(&self) -> Option<&T> {
        self.0.first()
    }

    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }

    pub fn insert(&mut self, id: T) -> bool {
        self.0.insert(id)
    }

    pub fn contains(&self, id: T) -> bool {
        self.0.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
}

impl<T: Copy + Ord> PortIdSet<T> {
    pub fn range(&self) -> Option<RangeInclusive<T>> {
        let Some(&min) = self.0.first() else {
            return None;
        };
        let Some(&max) = self.0.last() else {
            unreachable!()
        };
        Some(min..=max)
    }
}

impl<T: AsRef<isize>> Debug for PortIdSet<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(&self.0.iter().map(|id| id.as_ref()).collect::<Vec<_>>())
            .finish()
    }
}
