// This is free and unencumbered software released into the public domain.

use async_flow::model::{
    BlockDefinition, BlockName, InputPortId, OutputPortId, PortId, SystemBuilder, SystemDefinition,
    SystemValidationError,
};
use core::any::TypeId;
use std::borrow::Cow;

#[derive(Default)]
struct Block {
    inputs: Vec<InputPortId>,
    outputs: Vec<OutputPortId>,
    message_type: Option<TypeId>,
}

impl BlockName for Block {
    fn name(&self) -> Cow<'_, str> {
        "test".into()
    }
}

impl BlockDefinition for Block {
    fn inputs(&self) -> Vec<InputPortId> {
        self.inputs.clone()
    }

    fn outputs(&self) -> Vec<OutputPortId> {
        self.outputs.clone()
    }

    fn input_type(&self, _: InputPortId) -> Option<TypeId> {
        self.message_type
    }

    fn output_type(&self, _: OutputPortId) -> Option<TypeId> {
        self.message_type
    }
}

fn input(id: isize) -> InputPortId {
    id.try_into().unwrap()
}

fn output(id: isize) -> OutputPortId {
    id.try_into().unwrap()
}

fn registered_graph() -> SystemDefinition {
    let mut builder = SystemBuilder::new();
    builder.register_port(input(-1));
    builder.register_input(input(-2));
    builder.register_port(output(1));
    builder.register_output(output(2));
    builder.build()
}

#[test]
fn empty_and_standalone_ports_validate_without_exports_or_connections() {
    let empty = SystemDefinition::default();
    empty.validate().unwrap();
    assert_eq!(empty.inputs_range(), None);
    assert_eq!(empty.outputs_range(), None);

    let graph = registered_graph();
    graph.validate().unwrap();
    assert_eq!(graph.registered_inputs.len(), 2);
    assert_eq!(graph.registered_outputs.len(), 2);
    assert_eq!(graph.inputs_range(), Some(-2..=-1));
    assert_eq!(graph.outputs_range(), Some(1..=2));
    assert!(graph.inputs.is_empty());
    assert!(graph.outputs.is_empty());
}

#[test]
fn source_only_and_sink_only_block_graphs_validate() {
    for source in [false, true] {
        let mut builder = SystemBuilder::new();
        builder.register(Block {
            inputs: if source { vec![] } else { vec![input(-1)] },
            outputs: if source { vec![output(1)] } else { vec![] },
            message_type: Some(TypeId::of::<u8>()),
        });
        let graph = builder.build();
        graph.validate().unwrap();
        assert_eq!(
            graph.inputs_range(),
            if source { None } else { Some(-1..=-1) }
        );
        assert_eq!(
            graph.outputs_range(),
            if source { Some(1..=1) } else { None }
        );
        #[cfg(feature = "tokio")]
        assert!(matches!(
            graph.prepare(),
            Err(async_flow::tokio::SystemPrepareError::MissingProcessFactory { .. })
        ));
    }
}

#[test]
fn registrations_and_block_ids_both_contribute_to_ranges() {
    let mut builder = SystemBuilder::new();
    builder.register_input(input(isize::MIN));
    builder.register_output(output(isize::MAX));
    builder.register(Block {
        inputs: vec![input(-1)],
        outputs: vec![output(1)],
        message_type: None,
    });
    let graph = builder.build();
    assert_eq!(graph.inputs_range(), Some(isize::MIN..=-1));
    assert_eq!(graph.outputs_range(), Some(1..=isize::MAX));
    graph.validate().unwrap();
}

#[test]
fn exports_do_not_register_missing_ports() {
    for port in [PortId::Input(input(-1)), PortId::Output(output(1))] {
        let mut graph = SystemDefinition::default();
        match port {
            PortId::Input(id) => {
                graph.inputs.insert(id, TypeId::of::<u8>());
            },
            PortId::Output(id) => {
                graph.outputs.insert(id, TypeId::of::<u8>());
            },
        }
        assert_eq!(
            graph.validate(),
            Err(SystemValidationError::UnregisteredPort(port))
        );
    }
}

#[test]
fn connections_require_exact_registered_endpoints() {
    for (from, to, missing) in [
        (output(3), input(-1), PortId::Output(output(3))),
        (output(1), input(-3), PortId::Input(input(-3))),
    ] {
        let mut graph = registered_graph();
        graph.connections.insert((from, to), TypeId::of::<u8>());
        assert_eq!(
            graph.validate(),
            Err(SystemValidationError::UnregisteredPort(missing))
        );
    }
}

#[test]
fn removing_a_block_invalidates_its_edges() {
    let mut builder = SystemBuilder::new();
    builder.register_input(input(-1));
    builder.register(Block {
        outputs: vec![output(1)],
        ..Block::default()
    });
    let mut graph = builder.build();
    graph
        .connections
        .insert((output(1), input(-1)), TypeId::of::<u8>());
    graph.validate().unwrap();
    graph.blocks.clear();
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::UnregisteredPort(output(1).into()))
    );
}

#[test]
fn explicit_registration_can_alias_a_block_port() {
    let mut builder = SystemBuilder::new();
    builder.register_input(input(-1));
    builder.register_input(input(-1));
    builder.register(Block {
        inputs: vec![input(-1)],
        ..Block::default()
    });
    let graph = builder.build();
    assert_eq!(graph.registered_inputs.len(), 1);
    graph.validate().unwrap();
}

#[test]
fn duplicate_block_ports_are_rejected_within_and_across_blocks() {
    for inputs in [true, false] {
        for across_blocks in [true, false] {
            let mut builder = SystemBuilder::new();
            for _ in 0..if across_blocks { 2 } else { 1 } {
                let count = if across_blocks { 1 } else { 2 };
                builder.register(Block {
                    inputs: if inputs {
                        vec![input(-1); count]
                    } else {
                        vec![]
                    },
                    outputs: if inputs {
                        vec![]
                    } else {
                        vec![output(1); count]
                    },
                    message_type: None,
                });
            }
            let port = if inputs {
                input(-1).into()
            } else {
                output(1).into()
            };
            assert_eq!(
                builder.build().validate(),
                Err(SystemValidationError::DuplicatePort(port))
            );
        }
    }
}

#[test]
fn direct_field_edits_cannot_bypass_output_connection_uniqueness() {
    let mut graph = registered_graph();
    graph
        .connections
        .insert((output(1), input(-1)), TypeId::of::<u8>());
    graph
        .connections
        .insert((output(1), input(-2)), TypeId::of::<u8>());
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::AlreadyConnectedOutput(output(1)))
    );
}

#[test]
fn declared_block_types_are_checked_against_connections_and_exports() {
    for inputs in [true, false] {
        for exported in [true, false] {
            let mut builder = SystemBuilder::new();
            builder.register(Block {
                inputs: if inputs { vec![input(-1)] } else { vec![] },
                outputs: if inputs { vec![] } else { vec![output(1)] },
                message_type: Some(TypeId::of::<u8>()),
            });
            builder.register_input(input(-1));
            builder.register_output(output(1));
            let mut graph = builder.build();
            if exported {
                if inputs {
                    graph.inputs.insert(input(-1), TypeId::of::<u16>());
                } else {
                    graph.outputs.insert(output(1), TypeId::of::<u16>());
                }
            } else {
                graph
                    .connections
                    .insert((output(1), input(-1)), TypeId::of::<u16>());
            }
            assert_eq!(
                graph.validate(),
                Err(SystemValidationError::TypeMismatch {
                    port: if inputs {
                        input(-1).into()
                    } else {
                        output(1).into()
                    },
                    expected: TypeId::of::<u8>(),
                    actual: TypeId::of::<u16>(),
                })
            );
        }
    }
}

#[test]
fn exported_types_are_checked_against_connections() {
    for inputs in [true, false] {
        let mut graph = registered_graph();
        if inputs {
            graph.inputs.insert(input(-1), TypeId::of::<u8>());
        } else {
            graph.outputs.insert(output(1), TypeId::of::<u8>());
        }
        graph
            .connections
            .insert((output(1), input(-1)), TypeId::of::<u16>());
        assert_eq!(
            graph.validate(),
            Err(SystemValidationError::TypeMismatch {
                port: if inputs {
                    input(-1).into()
                } else {
                    output(1).into()
                },
                expected: TypeId::of::<u8>(),
                actual: TypeId::of::<u16>(),
            })
        );
    }
}

#[test]
fn fan_in_requires_consistent_inferred_types() {
    let mut graph = registered_graph();
    graph
        .connections
        .insert((output(1), input(-1)), TypeId::of::<u8>());
    graph
        .connections
        .insert((output(2), input(-1)), TypeId::of::<u8>());
    graph.validate().unwrap();
    graph
        .connections
        .insert((output(2), input(-1)), TypeId::of::<u16>());
    assert_eq!(
        graph.validate(),
        Err(SystemValidationError::TypeMismatch {
            port: input(-1).into(),
            expected: TypeId::of::<u8>(),
            actual: TypeId::of::<u16>(),
        })
    );
}

#[test]
fn cycles_are_structurally_valid() {
    let mut builder = SystemBuilder::new();
    for n in 1..=2 {
        builder.register(Block {
            inputs: vec![input(-n)],
            outputs: vec![output(n)],
            message_type: Some(TypeId::of::<u8>()),
        });
    }
    let mut graph = builder.build();
    graph
        .connections
        .insert((output(1), input(-2)), TypeId::of::<u8>());
    graph
        .connections
        .insert((output(2), input(-1)), TypeId::of::<u8>());
    graph.validate().unwrap();
}

#[cfg(feature = "tokio")]
#[test]
fn preparation_reads_block_metadata_once() {
    use std::{cell::Cell, rc::Rc};

    struct CountingBlock(Rc<Cell<usize>>);

    impl BlockName for CountingBlock {
        fn name(&self) -> Cow<'_, str> {
            "counting".into()
        }
    }

    impl BlockDefinition for CountingBlock {
        fn inputs(&self) -> Vec<InputPortId> {
            self.0.set(self.0.get() + 1);
            vec![input(-1)]
        }

        fn input_type(&self, _: InputPortId) -> Option<TypeId> {
            self.0.set(self.0.get() + 1);
            Some(TypeId::of::<async_flow::Message>())
        }
    }

    let calls = Rc::new(Cell::new(0));
    impl async_flow::tokio::ExecutableBlock for CountingBlock {
        fn create_process(
            &self,
            _: &mut async_flow::tokio::BlockPorts<'_>,
        ) -> Result<async_flow::tokio::ProcessFuture, async_flow::tokio::PortBindingError> {
            Ok(Box::pin(async { Ok(()) }))
        }
    }
    let mut builder = SystemBuilder::new();
    builder.register_executable(CountingBlock(Rc::clone(&calls)));
    let graph = builder.build();
    calls.set(0);
    graph.prepare().unwrap();
    assert_eq!(calls.get(), 2);
}
