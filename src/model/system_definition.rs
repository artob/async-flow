// This is free and unencumbered software released into the public domain.

use super::{InputId, OutputId, SystemBuilder};
use alloc::collections::BTreeSet;

/// A system definition.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemDefinition {
    //pub(crate) blocks: Vec<BlockDefinition>,
    pub(crate) connections: BTreeSet<(OutputId, InputId)>,
}

impl SystemDefinition {
    /// Returns a system builder.
    pub fn build() -> SystemBuilder {
        SystemBuilder::new()
    }
}
