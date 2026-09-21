// This is free and unencumbered software released into the public domain.

use core::ops::Bound;

/// Inclusive bounds on the total number of message payloads in a stream.
///
/// Control events and buffer slots are not messages for cardinality purposes.
/// Bounds are validated on construction. Connected port constraints are combined
/// by intersection; a finite maximum of zero describes an empty stream.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Cardinality {
    min: usize,
    max: Option<usize>,
}

impl Cardinality {
    /// Zero or more messages, with no message-count constraint.
    pub const UNLIMITED: Self = Self { min: 0, max: None };

    /// Zero or one message.
    pub const ONESHOT: Self = Self {
        min: 0,
        max: Some(1),
    };

    /// Creates inclusive bounds, returning `None` if the minimum exceeds the maximum.
    pub const fn new(min: usize, max: Option<usize>) -> Option<Self> {
        if let Some(max) = max
            && min > max
        {
            return None;
        }
        Some(Self { min, max })
    }

    /// Converts signed const-generic limits into validated bounds.
    ///
    /// `max == -1` means unlimited. The argument order matches port generics.
    ///
    /// # Panics
    ///
    /// Panics if `min < 0`, `max < -1`, or a finite `max` is less than `min`.
    pub const fn from_limits(max: isize, min: isize) -> Self {
        assert!(min >= 0, "minimum cardinality must be nonnegative");
        assert!(max >= -1, "maximum cardinality must be -1 or nonnegative");
        assert!(
            max == -1 || min <= max,
            "minimum cardinality exceeds maximum"
        );
        Self {
            min: min as usize,
            max: if max == -1 { None } else { Some(max as usize) },
        }
    }

    /// Returns the inclusive minimum message count.
    pub const fn min(self) -> usize {
        self.min
    }

    /// Returns the inclusive maximum message count, or `None` if unlimited.
    pub const fn max(self) -> Option<usize> {
        self.max
    }

    /// Reports whether neither a positive minimum nor a finite maximum is imposed.
    pub const fn is_unconstrained(self) -> bool {
        self.min == 0 && self.max.is_none()
    }

    /// Returns the equivalent standard-library bounds.
    pub const fn bounds(self) -> (Bound<usize>, Bound<usize>) {
        (
            Bound::Included(self.min),
            match self.max {
                Some(max) => Bound::Included(max),
                None => Bound::Unbounded,
            },
        )
    }

    /// Intersects two constraints, returning `None` for disjoint ranges.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let max = match (self.max, other.max) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, None) | (None, a) => a,
        };
        Self::new(self.min.max(other.min), max)
    }

    /// Adds producer ranges for fan-in, returning `None` on arithmetic overflow.
    ///
    /// An unlimited producer makes the aggregate maximum unlimited.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            min: self.min.checked_add(other.min)?,
            max: match (self.max, other.max) {
                (Some(a), Some(b)) => Some(a.checked_add(b)?),
                _ => None,
            },
        })
    }
}

impl Default for Cardinality {
    fn default() -> Self {
        Self::UNLIMITED
    }
}
