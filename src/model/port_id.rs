// This is free and unencumbered software released into the public domain.

/// A input or output port identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PortId {
    Input(InputPortId),
    Output(OutputPortId),
}

impl PortId {
    pub fn as_isize(&self) -> isize {
        match self {
            PortId::Input(id) => id.0,
            PortId::Output(id) => id.0,
        }
    }

    pub fn as_usize(&self) -> usize {
        self.as_isize() as _
    }
}

impl TryFrom<isize> for PortId {
    type Error = &'static str;

    fn try_from(id: isize) -> Result<Self, Self::Error> {
        if id < 0 {
            Ok(Self::Input(InputPortId(id)))
        } else if id > 0 {
            Ok(Self::Output(OutputPortId(id)))
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

/// An input port identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InputPortId(pub(crate) isize);

impl InputPortId {
    #[doc(hidden)]
    pub fn index(&self) -> usize {
        self.0.unsigned_abs() - 1
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
        input.0.unsigned_abs()
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

/// An output port identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutputPortId(pub(crate) isize);

impl OutputPortId {
    #[doc(hidden)]
    pub fn index(&self) -> usize {
        (self.0 as usize) - 1
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
        input.0 as usize
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
