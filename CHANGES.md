# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

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

### Added

- Backend-neutral graph validation for port IDs, block ownership, endpoint
  registration, output connection uniqueness, and message-type consistency.
- Optional per-port message-type metadata on `BlockDefinition`.
- `InputPortState::Ended` records receipt of a terminal disconnect marker and
  maps to `PortState::Disconnected`. It releases the receiver, so raw receiver
  access through `AsRef`/`AsMut` is no longer available after that marker.
- Bounded port lifecycle, backpressure, sender-clone, and cancellation tests,
  including outstanding-permit and payload-drop behavior.

### Fixed

- Fixed backend-neutral builds and feature-gated README doctests.
- Declared example feature requirements and qualified Tokio backend imports.
- Graph preparation handles empty, one-sided, manually registered, and sparse-ID
  graphs using dense port storage. Unsupported fan-in and non-`Message`
  connections return explicit errors instead of being silently miswired.
- Tokio `Port` trait implementations now forward buffer-capacity queries to
  their concrete endpoints.

## 0.1.5 - 2026-01-27

## 0.1.4 - 2026-01-27

## 0.1.3 - 2026-01-26

## 0.1.2 - 2026-01-26

## 0.1.1 - 2026-01-22

## 0.1.0 - 2026-01-19
