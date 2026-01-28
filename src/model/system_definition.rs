// This is free and unencumbered software released into the public domain.

use super::{BlockDefinition, InputPortId, OutputPortId, PortIdSet, SystemBuilder};
use alloc::{collections::BTreeMap, rc::Rc, vec::Vec};
use core::{any::TypeId, fmt::Debug, ops::RangeInclusive};

/// A system definition.
#[derive(Clone, Default)]
pub struct SystemDefinition {
    pub inputs: BTreeMap<InputPortId, TypeId>,
    pub outputs: BTreeMap<OutputPortId, TypeId>,
    pub blocks: Vec<BlockHandle>,
    pub connections: BTreeMap<(OutputPortId, InputPortId), TypeId>,
}

impl SystemDefinition {
    /// Returns a system builder.
    pub fn build() -> SystemBuilder {
        SystemBuilder::new()
    }

    pub(crate) fn push_block<T: BlockDefinition + 'static>(&mut self, block: &Rc<T>) {
        self.blocks.push(BlockHandle(Rc::clone(&block) as _));
    }

    pub fn inputs_range(&self) -> Option<RangeInclusive<InputPortId>> {
        let ranges = self
            .blocks
            .iter()
            .filter_map(|block| block.inputs_range())
            .collect::<Vec<_>>();
        let min = ranges.iter().map(|r| *r.start()).min()?;
        let max = ranges.iter().map(|r| *r.end()).max()?;
        Some(min..=max)
    }

    pub fn outputs_range(&self) -> Option<RangeInclusive<OutputPortId>> {
        let ranges = self
            .blocks
            .iter()
            .filter_map(|block| block.outputs_range())
            .collect::<Vec<_>>();
        let min = ranges.iter().map(|r| *r.start()).min()?;
        let max = ranges.iter().map(|r| *r.end()).max()?;
        Some(min..=max)
    }
}

impl Debug for SystemDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SystemDefinition")
            .field(
                "inputs",
                &self
                    .inputs
                    .iter()
                    .map(|(id, typ)| (id.0, typ))
                    .collect::<Vec<_>>(),
            )
            .field(
                "outputs",
                &self
                    .outputs
                    .iter()
                    .map(|(id, typ)| (id.0, typ))
                    .collect::<Vec<_>>(),
            )
            .field("blocks", &self.blocks)
            .field(
                "connections",
                &self
                    .connections
                    .iter()
                    .map(|((from, to), typ)| ((from.0, to.0), typ))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Clone)]
pub struct BlockHandle(Rc<dyn BlockDefinition>);

impl BlockHandle {
    pub fn inputs_range(&self) -> Option<RangeInclusive<InputPortId>> {
        let inputs = self.0.inputs();
        let Some(&min) = inputs.iter().min() else {
            return None;
        };
        let Some(&max) = inputs.iter().max() else {
            unreachable!()
        };
        Some(min..=max)
    }

    pub fn outputs_range(&self) -> Option<RangeInclusive<OutputPortId>> {
        let outputs = self.0.outputs();
        let Some(&min) = outputs.iter().min() else {
            return None;
        };
        let Some(&max) = outputs.iter().max() else {
            unreachable!()
        };
        Some(min..=max)
    }
}

impl Debug for BlockHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inputs = PortIdSet::from(&self.0.inputs());
        let outputs = PortIdSet::from(&self.0.outputs());
        f.debug_struct(&self.0.name())
            .field("inputs", &inputs)
            .field("outputs", &outputs)
            .finish()
    }
}
