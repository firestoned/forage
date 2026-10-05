<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Dependencies: own it if it is 500 lines or less

> We would rather maintain the code ourselves than keep upgrading crates or
> wait on upstream projects to fix bugs. ([ADR-0006](../../docs/adr/0006-own-small-dependencies.md))

## The rule

Before adding **any** dependency (runtime or dev), and whenever an existing
one is touched:

1. **List exactly what forage uses** from it: the functions, types, macros
   and behaviors, not the crate's whole surface. `rg` every use site.
2. **Estimate the production Rust we would own** to provide exactly that
   (excluding tests, blank lines and comments).
3. **500 lines or less: write it here** as a module in `src/`, test-first
   (`tdd-workflow` skill), under the ADR-0005 coverage gate, and do not add
   or keep the crate.
4. **More than 500 lines**: the crate may be used. Record the measurement
   and the reason in an ADR (what is used, upstream size, our estimate).

## When the rule does not apply

Some code must not be hand-rolled even when small: cryptography, TLS,
compression, anything security-critical where a subtle bug is worse than an
upgrade. Keep the crate and say why in the ADR.

## Owning the code properly

- One module per replaced capability, named for the capability (`json.rs`,
  `cli.rs`), not the crate it replaced.
- Unit tests in `<module>_tests.rs` cover every branch (100% line and
  function gate).
- When replacing a crate whose output users see, capture golden files from
  the old implementation **before** switching, and require byte-identical
  output (or document every difference).
- Record the swap in `.claude/CHANGELOG.md` with the before/after line counts.

## Checklist (paste into the work)

- [ ] Use sites listed (`rg`)
- [ ] Owned-code estimate written down; ≤ 500 → own it, > 500 → ADR
- [ ] Tests first; `make coverage` green
- [ ] Golden output unchanged (or differences documented)
- [ ] Crate removed from `Cargo.toml`; `Cargo.lock` updated; `make cargo-machete` and `make cargo-deny` green
