---
name: cargo-quality
description: Run the mandatory Rust quality gate (cargo fmt + clippy -D warnings + cargo test). Use after adding or modifying ANY .rs file, before committing Rust changes, and at the end of EVERY task involving Rust code (NON-NEGOTIABLE). A Rust task is not complete until all three pass.
---

<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# cargo-quality

The non-negotiable quality gate for any Rust change. All three commands must
exit 0: no warnings, no test failures: before a task is complete.

## Steps

```bash
# 1. Format
cargo fmt

# 2. Lint with strict warnings (fix ALL warnings)
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions

# 3. Test (ALL tests must pass)
cargo test

# 4. Security audit (optional, if installed)
cargo audit 2>/dev/null || true
```

If `cargo` is not on PATH, it lives in `~/.cargo/bin`. Build on slate (`rsync -az --delete --exclude target ./ slate:builds/forage/`), not the mac.

Then the coverage gate (ADR-0005): `make coverage` must pass (zero unexecuted
lines, 100% of functions). New code arrives with the tests that execute it.

If the change affects what forage prints, also run `make e2e-schema` (ADR-0003).

The pedantic set in step 2 still has a backlog on the initial-release code
(roadmap 02); do not add new findings, and fix any you touch.

## After the gate passes, also verify

1. **Rustdoc accuracy**: comments on changed functions match actual behavior
   (`# Arguments`, `# Returns`, `# Errors`).
2. **Tests match the change**: new public functions have tests, deleted
   functions have tests removed, changed behavior has updated assertions
   (see `tdd-workflow` skill).
3. **Docs**: `.claude/CHANGELOG.md` entry written (`update-changelog` skill)
   and `README.md` / affected docs updated (`rules/documentation.md`).

## Verification

All three commands exit with code 0. No clippy warnings, no test failures.
