# moveref

Types and traits for safe C++-style placement initialization and move semantics.

[API documentation](https://docs.rs/moveref) · [CI](https://github.com/silvanshade-org/moveref/actions/workflows/ci.yaml)

Originally based on [google/moveit](https://github.com/google/moveit).

## Status

This library provides in-place construction and owning move references for Rust and C++ interop. Its lifecycle tests run under Miri in CI, alongside ordinary tests, Clippy, rustdoc, and formatter gates.

## Known lifecycle defects

- A failed `Slot::try_emplace` initializer leaves its storage marked as leaking. Dropping that storage aborts or panics; do not use this fallible path until the defect is resolved.
- `SlotStorageKind::Drop` destroys a directly owned referent twice. Use `Keep` for directly stored values; `Drop` remains needed for distinct backing allocations.
