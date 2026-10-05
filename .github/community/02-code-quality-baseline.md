<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 02: Code-quality baseline

> Bring the initial-release code up to the repository's own rules
> (`.claude/rules/rust-style.md`, `.claude/rules/testing.md`), so the
> `cargo-quality` skill passes as written.

Audited 2026-10-05. Behavior-preserving except where noted; TDD only.

## Tasks

- [x] `cargo fmt --check` was failing on the initial release; formatted
      2026-10-05 (the new `format` CI job would otherwise fail every PR).
- [x] Test file naming: `src/mapper_test.rs` renamed to
      `src/mapper_tests.rs` per `rules/testing.md`.
- [ ] **Pedantic clippy** (`make clippy-pedantic`, the command in the
      `cargo-quality` skill) reports 14 findings. CI runs plain
      `-D warnings`, which is clean. Notable ones:
  - six `u32 as i32` casts that can wrap (TTLs and SOA timers from the zone
    file into `i32` spec fields). A TTL above `i32::MAX` would become
    negative. Decide: clamp, reject with a warning, or widen the field (bindy's
    schema is `int32`, so clamp-or-reject). This one changes behavior.
  - `Mapper::map` is 146 lines (limit 100): split per zone.
  - `map().unwrap_or_else()`, single-arm `match`, `let...else`, unnested
    or-patterns, missing doc backticks, an unused `self`.
  Once clean, switch the CI `clippy` target to the pedantic set so it cannot
  regress.
- [ ] **Magic numbers** in `Mapper::build_dns_zone`'s fallback SOA (3600,
      600, 604800, 86400) become named constants (`rules/rust-style.md`).
- [ ] **Repeated string literals**: kind names (`"DNSZone"`, `"ARecord"` …)
      and the `"forage"` name prefix become constants in `src/crd.rs`.
- [ ] **Test layout**: `src/mapper_tests.rs` uses `use super::*` at the top
      level; move it under `#[cfg(test)] mod tests { use super::super::*; }`
      like `src/crd_tests.rs` for consistency.
- [x] **Coverage of the mapper's record path**: per-type unit tests in
      `src/mapper_tests.rs` (2026-10-05), with the whole crate under a 100%
      line and function gate (ADR-0005).

## Definition of done

`make clippy-pedantic` exits 0 and CI enforces it; no magic numbers or
repeated literals in `src/` outside tests.
