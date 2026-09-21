# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- Definition preparation requires executable registration for blocks, creates
  fresh unpolled process futures, and starts them in `System::execute()`. Explicit
  `System::spawn()` still starts tasks immediately. Untaken boundary endpoints
  are dropped before startup; callers must drive taken endpoints concurrently.
- With `alloc`, `Message` now uses `valuand::Value<Box<dyn Any + Send + Sync>>`
  so it can cross runtime threads. Erased non-sendable values are no longer accepted.
- Typed builder connections and descriptor exports require `Send + 'static`
  payloads and retain Tokio constructors. `PortExport` adds a Tokio-gated factory
  field; use `PortExport::new(id, type_id).with_cardinality(bounds)` for raw exports.
  `SystemDefinition` adds a Tokio-gated `channel_factories` map.
- Merged inputs consume source-local disconnect markers and continue receiving
  other producers. Raw access is prohibited on merged/grouped endpoints even
  when message cardinality is unlimited.

- Raised the minimum supported Rust version from 1.85 to 1.97.
- `SystemDefinition::prepare()` now returns `Result<System, SystemPrepareError>`;
  `System::try_from(&definition)` replaces the infallible conversion.
- System definitions retain explicit registrations in `registered_inputs` and
  `registered_outputs`. Struct literals must include these fields or use
  `..Default::default()`.
- Tokio inputs treat a received `Disconnect` event as terminal EOF, discard
  trailing events, and disconnect all senders. `Connect` remains informational.
  Graceful input disconnection drains accepted events and waits for outstanding
  Tokio permits; output closure remains handle-local.
- Tokio `Channel`, `Inputs`, and `Outputs` accept a defaulted `MIN` parameter.
  `bounded()` and `pair()` preserve their const-generic bounds. Use
  `Channel::<T>::oneshot()` for zero-or-one messages or
  `Channel::<T, 1, 1>::bounded(1)` for exactly one required message.
- Raw Tokio conversions and `AsRef`/`AsMut` implementations are limited to
  default-typed endpoints. Raw access also rejects effective constrained bounds
  installed by system preparation, preventing quota bypass/reset.
- `OutputPort` now extends `Port`; `InputPort` requires `disconnect()`.
  `RecvError` is now an enum (`Unavailable` replaces its former unit value).
- Builder registration/export accepts `PortRegistration`/`PortExport` conversions
  to preserve descriptor bounds; raw IDs and ID/type tuples remain supported.
  `connect()` infers independent input/output const parameters; explicit
  turbofish calls must supply those parameters or switch to inference. Definitions add
  the `cardinalities` field; struct literals can use `..Default::default()`.

### Added

- Tokio `ExecutableBlock`, `ProcessFuture`, `BlockPorts`, checked channel factories,
  and typed external boundary accessors. Process factories remain attached to
  block handles across definition clones/reordering. Prepared systems are `Send`.
- Typed, fair fan-in with one bounded queue per source, per-source FIFO ordering,
  producer-specific cardinality checks, and shared-budget minimum reservations.
- Preparation/binding errors for missing factories, type/ownership/cardinality
  mismatches, unclaimed connected ports, and internally connected exports.
  `FanInBudgetExhausted` and `ProducerCardinalityUnderflow` report fan-in failures.
- The `defined_system` example demonstrates typed executable factories and fan-in.

- Backend-neutral graph validation for port IDs, block ownership, endpoint
  registration, output connection uniqueness, and message-type consistency.
- Optional per-port message-type metadata on `BlockDefinition`.
- Validated `Cardinality` ranges, block cardinality metadata, intersected port
  constraints, and aggregate producer-range validation for structural fan-in.
- Explicit port-ID `magnitude()` accessors and `PortId::from_usize()` for decoding
  direction-preserving unsigned keys. Existing unsigned conversions retain their
  meanings: full IDs encode direction, while typed IDs yield local magnitudes.
- Shared, cancellation-safe sender quotas; finite streams reach EOF at their
  maximum even with live senders. Minimum shortfalls are reported once at EOF or
  a disconnect marker; explicit input closure remains an abort.
- `SendError::CardinalityExceeded`, `RecvError::CardinalityUnderflow`, and
  effective cardinality queries on concrete ports and `Port` trait objects.
- `InputPortState::Ended` records receipt of a terminal disconnect marker and
  maps to `PortState::Disconnected`. It releases the receiver, so raw receiver
  access through `AsRef`/`AsMut` is no longer available after that marker.
- Bounded port lifecycle, backpressure, sender-clone, and cancellation tests,
  including outstanding-permit and payload-drop behavior.

### Fixed

- Definitions now execute registered block processes and support arbitrary
  registered sendable payload types and fan-in. The old `UnsupportedFanIn` and
  `UnsupportedMessageType` preparation errors are replaced by factory/binding errors.

- Fixed backend-neutral builds and feature-gated README doctests.
- Declared example feature requirements and qualified Tokio backend imports.
- System preparation handles empty, one-sided, manually registered, and sparse-ID
  definitions with storage proportional to the number of actual ports.
- Tokio `Port` trait implementations now forward buffer-capacity queries to
  their concrete endpoints.
- One-shot connections enforce a shared maximum of one payload. `Connection`
  applies to every channel cardinality. Cloning outputs and creating default
  runtime endpoints no longer require payload `Clone`/`Default` implementations.
- Port-ID deserialization now rejects zero, wrong-sign, and out-of-range values,
  including values under contradictory `PortId` tags. Valid signed/newtype and
  externally tagged representations are preserved.
- Descriptor ID allocation stops at exhaustion instead of wrapping into invalid
  or reused IDs; requesting another ID then panics.

## 0.1.5 - 2026-01-27

## 0.1.4 - 2026-01-27

## 0.1.3 - 2026-01-26

## 0.1.2 - 2026-01-26

## 0.1.1 - 2026-01-22

## 0.1.0 - 2026-01-19
