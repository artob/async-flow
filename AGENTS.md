# Working on async-flow

Rust 2024 library for async flow-based programming. Advertised MSRV: 1.85;
verify dependency compatibility when changing features or dependencies.

## Code map
- `src/io/`, `src/error/`: backend-neutral traits, events, states, errors;
  reexported at the crate root.
- `src/model/`: graph definitions, signed port IDs, builder. Its `Inputs` and
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
- Input IDs are negative, output IDs positive, zero invalid. The builder
  permits each output only one connection.
- Tokio input `disconnect()` retains buffered events; `close()` discards them.
  Spawning system tasks requires an active Tokio runtime; `execute()` joins them.
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
- `--no-default-features` fails because `SystemDefinition::prepare` references
  Tokio unconditionally. `--all-features` fails in Flume implementations and
  Tokio serial-scheduler imports. Default checks pass; Clippy emits warnings.
- Graph preparation panics on empty graphs and does not schedule blocks.
  Blocking send/recv methods are `todo!()`.
- `Channel::oneshot` only sets buffer capacity to one; cardinality is not
  enforced. `UNLIMITED` is not an unbounded-buffer constructor.
- `tests/tokio_system.rs` covers system execution/shutdown; `benches/` is a placeholder.
- `rust-version` is commented out; the locked SQLx dependency tree includes
  ICU crates requiring Rust 1.86, above the advertised MSRV.
