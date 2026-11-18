---
id: 0005
name: rename-chorus
status: done
created: 2026-11-18
---

# 0005: Rename Library to Chorus

Rename `shiver_core::Library` to `Chorus` and `library.rs` to `chorus.rs`, so
the file no longer looks like a duplicate of `lib.rs`. No behaviour change.
Human-facing overview: [onepager.html](onepager.html).

## Mapping

Specs 0001, 0002 and 0004 are done and stay as written. Where they say:

| Old | New |
|---|---|
| `Library` | `Chorus` |
| `Library::load`, `Library::pick`, … | `Chorus::load`, `Chorus::pick`, … |
| `crates/shiver-core/src/library.rs` | `crates/shiver-core/src/chorus.rs` |
| test `library::tests::<name>` | test `chorus::tests::<name>` |

## Requirements

### R1: Rename the type and module

`Library` becomes `Chorus`, the module `library` becomes `chorus`, and
`lib.rs` re-exports `Chorus` instead of `Library`. No alias for the old name.

**Acceptance:**
- [x] `grep -rn "Library" crates` finds nothing.
- [x] `crates/shiver-core/src/library.rs` no longer exists; `chorus.rs` does.

### R2: Rename variables and docs

Variables, parameters, helper names and doc comments that refer to the
library (`library`, `lib`) use `chorus`. Mentions of the Rust library crate
itself (e.g. "library crate") are unchanged.

**Acceptance:**
- [x] `grep -rnw "library" crates` finds only mentions of the library crate.
- [x] Review: no `lib` variable names left for a `Chorus`.

### R3: Same behaviour

Every test from before still exists under its new path and passes. Test count
and coverage don't drop.

**Acceptance:**
- [x] `cargo test --workspace` passes with the same number of tests as before (82).
- [x] All CI checks pass locally.

## Out of scope

- Editing done specs.
- Any change in behaviour or output.
