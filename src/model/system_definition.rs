// This is free and unencumbered software released into the public domain.

use super::{BlockDefinition, InputId, OutputId, SystemBuilder};
use alloc::{borrow::Cow, collections::BTreeSet, rc::Rc, vec::Vec};
use core::fmt::Debug;

/// A system definition.
#[derive(Clone, Default)]
pub struct SystemDefinition {
    pub(crate) blocks: Vec<Rc<dyn BlockDefinition>>,
    pub(crate) connections: BTreeSet<(OutputId, InputId)>,
}

impl SystemDefinition {
    /// Returns a system builder.
    pub fn build() -> SystemBuilder {
        SystemBuilder::new()
    }
}

impl Debug for SystemDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct BlockName<'a>(Cow<'a, str>);
        impl<'a> Debug for BlockName<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(&self.0).finish()
            }
        }

        let block_names = self
            .blocks
            .iter()
            .map(|block| BlockName(block.name()))
            .collect::<Vec<_>>();
        f.debug_struct("SystemDefinition")
            .field("blocks", &block_names)
            .field("connections", &self.connections)
            .finish()
    }
}
