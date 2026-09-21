// This is free and unencumbered software released into the public domain.

//! Structural definitions of systems, blocks, and ports.
//!
//! A system's graph records its blocks and their port connections. The port
//! descriptors in this module identify connection points; runtime backends
//! provide the endpoints that exchange messages and the tasks that execute blocks.

mod block_definition;
pub use block_definition::*;

mod inputs;
pub use inputs::*;

mod outputs;
pub use outputs::*;

mod port_direction;
pub use port_direction::*;

mod port_id;
pub use port_id::*;

mod port_id_map;
pub use port_id_map::*;

mod port_id_set;
pub use port_id_set::*;

mod port_registration;
pub use port_registration::*;

mod port_export;
pub use port_export::*;

mod system_definition;
pub use system_definition::*;

mod system_builder;
pub use system_builder::*;

mod system_validation_error;
pub use system_validation_error::*;
