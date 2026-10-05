---
name: pre-commit-checklist
description: The mandatory gate before EVERY commit. Walks the Rust, output-contract, ADD and always-on checklists (tests, clippy, e2e, docs, roadmaps, changelog, no secrets). A task is NOT complete until every applicable box is green.
---

<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# pre-commit-checklist

Run before committing any change. Every applicable box must pass.

## If ANY `.rs` file was modified

- [ ] Tests updated/added/deleted to match changes (TDD: see `tdd-workflow`)
- [ ] All new public functions have tests
- [ ] All deleted functions have tests removed
- [ ] `make format-check` passes
- [ ] `make clippy` passes (and no new `make clippy-pedantic` findings)
- [ ] `cargo test` passes (ALL tests green)
- [ ] `make coverage` passes: zero unexecuted lines, 100% of functions (ADR-0005)
- [ ] A known bug gets a correct-behavior test marked `#[ignore = "... (roadmap NN)"]`, never a test that asserts the wrong behavior
- [ ] Rustdoc comments on all public items, accurate to actual behavior

## If what forage emits changed (`src/crd.rs`, `src/mapper.rs`, `src/main.rs`)

- [ ] `src/crd_tests.rs` key-set tests match bindy's field names
- [ ] `tests/cli_tests.rs` / `tests/fixtures/` cover the new behavior
- [ ] `make e2e-schema` passes (`verify-crd-sync` skill)
- [ ] README mapping table and examples updated

## If the change was architecturally significant (ADD)

- [ ] ADR in `docs/adr/` (metadata bullets format)
- [ ] CALM model updated; `make calm-validate` + `make calm-docs` + `make calm-docs-check` clean
- [ ] Threat-model pass done, header stamp bumped

## Always

- [ ] `.claude/CHANGELOG.md` updated with **Author:** line (MANDATORY: `update-changelog`)
- [ ] Roadmap detail doc AND `ROADMAPS.md` updated for anything completed
- [ ] Every new file carries the `Copyright (c) 2025 Erick Bourgeois, firestoned` / `SPDX-License-Identifier: Apache-2.0` header where the format allows comments
- [ ] No secrets, tokens, credentials, real hostnames or IP addresses
      committed (fixtures use RFC 2606 names and RFC 5737 / RFC 3849 addresses)
- [ ] No `.unwrap()` in production code
- [ ] No em-dashes in prose you wrote: `rg -n '—' <files you touched>`

## Commit shape

Commits only when Erick asks, always `git commit -s -S -m "<message>"`:
authored as Erick, never Claude, no co-author trailers or generated-by footers.

## Verification

Every checked box above passes. A task is NOT complete until the full
checklist is green.
