# Working on async-flow

Rust 2024 library for async flow-based programming. MSRV: 1.97;
verify dependency compatibility when changing features or dependencies.

## Terminology
- Follow `README.md`'s glossary and the **Flux Theory** column of the
  [FBP terminology cross-reference](https://github.com/flux-doctrine/awesome-fbp#concepts).
  Classical → project: network → system; subnet → subsystem; component → block;
  information packet (IP) → message; initial information packet (IIP) → property.
- A system comprises connected blocks; blocks exchange messages through ports.
  A process is a block's execution, represented here by a Tokio task. Keep
  blocks, processes/tasks, and runtime threads distinct.
- A graph represents a system's structure. Distinguish domain messages from
  the concrete `Message` alias. Do not infer classical FBP runtime/ownership
  rules from its terminology.

## Code map
- `src/io/`, `src/error/`: backend-neutral traits, events, states, errors;
  reexported at the crate root.
- `src/model/`: system/block definitions, signed port IDs, builder. Its `Inputs` and
  `Outputs` are descriptors, distinct from runtime ports.
- `src/tokio/`: runtime ports, bounded MPSC channels carrying `PortEvent<T>`,
  `System`/`JoinSet`, schedulers, stdio blocks.
- `src/flume/`: unfinished optional backend. `examples/`: runnable usage.

## Rules
- Work within this project; do not inspect parent directories.
- Preserve `#![no_std]` and `#![forbid(unsafe_code)]`. Use `core`/`alloc`;
  feature-gate `std` APIs and backend dependencies. Allocation is currently
  unconditional; the `alloc` flag selects richer message aliases.
- Default features are `all,std`; `all` selects `parallel,serial,stream,tokio`,
  not every optional integration.
- Use backend-qualified imports internally (`crate::tokio`, `crate::flume`).
  Root backend reexports exist only when exactly one backend is enabled.
- Follow private per-type modules with `pub use`, existing public-domain
  headers, and `.rustfmt.toml`.
- Input IDs are negative, output IDs positive, zero invalid, including in Serde.
  `PortId`'s `usize` encoding preserves direction; typed-ID conversions yield
  magnitudes. `index()` is not a dense runtime index. The builder permits each
  output only one connection.
- Cardinality counts lifetime payloads, not controls or buffer slots. Port bounds
  intersect; sender clones share quotas. Fan-in reserves producer minimums within
  the aggregate budget. Raw access requires unconstrained, non-grouped ports.
  `UNLIMITED` is not an unbounded-buffer constructor.
- Tokio input `disconnect()` drains accepted events; `close()` discards them.
  `Connect` is informational; receiving `Disconnect` terminates the connection
  and discards trailing events. Fan-in consumes source-local disconnect markers;
  other producers continue. Output `close()` affects only that handle.
- `register_executable` attaches factories to block handles. `prepare()` needs no
  runtime and returns unpolled `Send` futures; `execute()` starts them and joins.
  Explicit `spawn()` starts immediately. Exports must be internally unconnected;
  take boundary endpoints before execution and drive them concurrently.
- Typed connections/exports register payload constructors; raw `TypeId` metadata
  needs `register_message_type`. Missing factories and invalid bindings are errors.
- Aim for rustdoc on every public symbol; document new/changed APIs, including
  lifecycle, errors, panics, feature/runtime requirements, and useful examples.
  Prefer module/type rustdoc over README additions; expand README only when
  the information genuinely belongs at the entry point.
- Test changed runtime behavior: EOF, close/disconnect, buffered messages,
  backpressure, cancellation, and error propagation. Keep async tests bounded.
- For releases, synchronize `Cargo.toml`, `VERSION`, and `CHANGES.md`;
  regenerate `Cargo.lock` with Cargo.

## Checks
For Rust changes, run from the project root:
```sh
cargo fmt --all --check
cargo test --locked
cargo build --locked --examples
cargo clippy --locked --all-targets
```
For rustdoc changes: `cargo doc --locked --no-deps` and
`cargo test --locked --doc`. README Rust snippets are doctested via `src/lib.rs`.

For feature changes, run `cargo check --locked --lib` with affected feature
sets, including `--no-default-features`, `--no-default-features --features tokio`,
and `--all-features`. Distinguish existing failures from regressions.

## Known gaps — recheck when touched; update when fixed
- Flume-enabled builds, including `--all-features`, fail in unfinished Flume
  implementations. CI covers non-Flume feature sets. Clippy emits warnings.
- Blocking send/recv methods are `todo!()`. Prepared connections use capacity-one
  queues (one per producer in fan-in); buffer configuration is not yet exposed.
- `tests/` covers graph validation, port lifecycle/backpressure/cancellation, and
  system execution/shutdown; `benches/` is a placeholder.
