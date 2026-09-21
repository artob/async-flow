// This is free and unencumbered software released into the public domain.

use core::sync::atomic::{AtomicIsize, Ordering};

/// An input or output port identifier, with direction encoded by its sign.
///
/// Inputs are strictly negative, outputs strictly positive, and zero is invalid.
/// With `serde`, the representation is externally tagged (`{"input":-1}` or
/// `{"output":1}` in JSON). Deserialization validates both sign and `isize` range.
///
/// # Numeric representations
///
/// [`as_usize`](Self::as_usize) and `From<PortId> for usize` preserve direction
/// using the signed value's bit pattern. In contrast, conversion of a typed
/// [`InputPortId`] or [`OutputPortId`] to `usize` returns its direction-local
/// [`magnitude`](Self::magnitude). Magnitudes can coincide across directions.
/// Neither representation is a system's dense storage index.
///
/// ```
/// use async_flow::model::{InputPortId, PortId};
///
/// let input = InputPortId::try_from(-3)?;
/// let port = PortId::from(input);
/// assert_eq!(usize::from(input), 3);
/// assert_eq!(port.magnitude(), 3);
/// assert_eq!(port.as_isize(), -3);
/// assert_eq!(PortId::from_usize(port.as_usize())?, port);
/// assert_ne!(port.as_usize(), PortId::try_from(3)?.as_usize());
/// # Ok::<(), &'static str>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortId {
    /// An input port, identified by a strictly negative integer.
    Input(InputPortId),
    /// An output port, identified by a strictly positive integer.
    Output(OutputPortId),
}

impl PortId {
    /// Returns the signed identifier, preserving its direction.
    pub const fn as_isize(&self) -> isize {
        match self {
            PortId::Input(id) => id.0,
            PortId::Output(id) => id.0,
        }
    }

    /// Encodes the signed ID as a direction-preserving unsigned key.
    ///
    /// Outputs occupy `1..=isize::MAX`; inputs occupy the upper half of `usize`.
    /// For example, input `-1` becomes `usize::MAX`. This encoding depends on
    /// pointer width; Serde uses the signed ID, not this encoding.
    pub const fn as_usize(&self) -> usize {
        self.as_isize() as usize
    }

    /// Decodes the unsigned key produced by [`as_usize`](Self::as_usize).
    ///
    /// This interprets a direction-preserving bit pattern, not a typed ID's
    /// magnitude. Every nonzero `usize` maps to exactly one valid `PortId`.
    ///
    /// # Errors
    ///
    /// Returns an error for zero.
    pub fn from_usize(encoded: usize) -> Result<Self, &'static str> {
        Self::try_from(encoded as isize)
    }

    /// Returns the direction-local unsigned magnitude, including for `isize::MIN`.
    ///
    /// This matches `usize::from` on the wrapped typed ID and discards direction.
    pub const fn magnitude(&self) -> usize {
        self.as_isize().unsigned_abs()
    }
}

impl TryFrom<isize> for PortId {
    type Error = &'static str;

    fn try_from(id: isize) -> Result<Self, Self::Error> {
        if id < 0 {
            InputPortId::try_from(id).map(Self::Input)
        } else if id > 0 {
            OutputPortId::try_from(id).map(Self::Output)
        } else {
            Err("Port IDs cannot be zero")
        }
    }
}

impl From<InputPortId> for PortId {
    fn from(input: InputPortId) -> Self {
        PortId::Input(input)
    }
}

impl From<OutputPortId> for PortId {
    fn from(input: OutputPortId) -> Self {
        PortId::Output(input)
    }
}

impl From<PortId> for isize {
    fn from(input: PortId) -> isize {
        input.as_isize()
    }
}

impl From<PortId> for usize {
    fn from(input: PortId) -> usize {
        input.as_usize()
    }
}

impl AsRef<isize> for PortId {
    fn as_ref(&self) -> &isize {
        match self {
            PortId::Input(id) => &id.0,
            PortId::Output(id) => &id.0,
        }
    }
}

impl core::fmt::Display for PortId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PortId::Input(id) => write!(f, "{}", id),
            PortId::Output(id) => write!(f, "{}", id),
        }
    }
}

/// An input port identifier represented by a strictly negative `isize`.
///
/// `TryFrom<isize>` and Serde deserialization reject zero and positive values.
/// Serde preserves the signed newtype representation (a negative number in JSON).
/// `From<InputPortId> for usize` returns the unsigned [`magnitude`](Self::magnitude),
/// unlike the direction-preserving encoding of [`PortId::as_usize`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct InputPortId(pub(crate) isize);

impl InputPortId {
    /// Returns the one-based, direction-local unsigned magnitude.
    ///
    /// Uses unsigned absolute value so `isize::MIN` is representable without overflow.
    pub const fn magnitude(&self) -> usize {
        self.0.unsigned_abs()
    }

    /// Returns the magnitude minus one (`-1` maps to zero).
    ///
    /// This is an arithmetic ordinal within the input-ID space, not a dense
    /// system index. IDs can be sparse; runtime preparation uses explicit ID maps.
    pub const fn index(&self) -> usize {
        self.magnitude() - 1
    }

    pub(crate) fn next() -> Self {
        static COUNTER: AtomicIsize = AtomicIsize::new(-1);
        Self::allocate(&COUNTER).expect("input port ID space exhausted")
    }

    fn allocate(counter: &AtomicIsize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| {
                // Zero marks exhaustion after issuing the last valid negative ID.
                (id < 0).then(|| id.checked_sub(1).unwrap_or(0))
            })
            .ok()
            .map(Self)
    }
}

impl TryFrom<isize> for InputPortId {
    type Error = &'static str;

    fn try_from(id: isize) -> Result<Self, Self::Error> {
        if id < 0 {
            Ok(InputPortId(id))
        } else {
            Err("Input port IDs must be negative integers")
        }
    }
}

impl From<InputPortId> for isize {
    fn from(input: InputPortId) -> isize {
        input.0
    }
}

impl From<InputPortId> for usize {
    fn from(input: InputPortId) -> usize {
        input.magnitude()
    }
}

impl AsRef<isize> for InputPortId {
    fn as_ref(&self) -> &isize {
        &self.0
    }
}

impl core::fmt::Display for InputPortId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An output port identifier represented by a strictly positive `isize`.
///
/// `TryFrom<isize>` and Serde deserialization reject zero and negative values.
/// Serde preserves the signed newtype representation (a positive number in JSON).
/// `From<OutputPortId> for usize` returns the [`magnitude`](Self::magnitude).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OutputPortId(pub(crate) isize);

impl OutputPortId {
    /// Returns the one-based, direction-local unsigned magnitude.
    pub const fn magnitude(&self) -> usize {
        self.0 as usize
    }

    /// Returns the magnitude minus one (`1` maps to zero).
    ///
    /// This is an arithmetic ordinal within the output-ID space, not a dense
    /// system index. IDs can be sparse; runtime preparation uses explicit ID maps.
    pub const fn index(&self) -> usize {
        self.magnitude() - 1
    }

    pub(crate) fn next() -> Self {
        static COUNTER: AtomicIsize = AtomicIsize::new(1);
        Self::allocate(&COUNTER).expect("output port ID space exhausted")
    }

    fn allocate(counter: &AtomicIsize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| {
                // Zero marks exhaustion after issuing the last valid positive ID.
                (id > 0).then(|| id.checked_add(1).unwrap_or(0))
            })
            .ok()
            .map(Self)
    }
}

impl TryFrom<isize> for OutputPortId {
    type Error = &'static str;

    fn try_from(input: isize) -> Result<Self, Self::Error> {
        if input > 0 {
            Ok(OutputPortId(input))
        } else {
            Err("Output port IDs must be positive integers")
        }
    }
}

impl From<OutputPortId> for isize {
    fn from(input: OutputPortId) -> isize {
        input.0
    }
}

impl From<OutputPortId> for usize {
    fn from(input: OutputPortId) -> usize {
        input.magnitude()
    }
}

impl AsRef<isize> for OutputPortId {
    fn as_ref(&self) -> &isize {
        &self.0
    }
}

impl core::fmt::Display for OutputPortId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InputPortId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Preserve the newtype name and representation in non-JSON formats too.
        #[derive(serde::Deserialize)]
        #[serde(rename = "InputPortId")]
        struct Repr(isize);

        Self::try_from(Repr::deserialize(deserializer)?.0).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for OutputPortId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename = "OutputPortId")]
        struct Repr(isize);

        Self::try_from(Repr::deserialize(deserializer)?.0).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_allocation_stops_after_the_last_negative_id() {
        let counter = AtomicIsize::new(isize::MIN + 1);
        assert_eq!(
            InputPortId::allocate(&counter),
            Some(InputPortId(isize::MIN + 1))
        );
        assert_eq!(
            InputPortId::allocate(&counter),
            Some(InputPortId(isize::MIN))
        );
        for _ in 0..3 {
            assert_eq!(InputPortId::allocate(&counter), None);
            assert_eq!(counter.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn output_allocation_stops_after_the_last_positive_id() {
        let counter = AtomicIsize::new(isize::MAX - 1);
        assert_eq!(
            OutputPortId::allocate(&counter),
            Some(OutputPortId(isize::MAX - 1))
        );
        assert_eq!(
            OutputPortId::allocate(&counter),
            Some(OutputPortId(isize::MAX))
        );
        for _ in 0..3 {
            assert_eq!(OutputPortId::allocate(&counter), None);
            assert_eq!(counter.load(Ordering::Acquire), 0);
        }
    }
}
