<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0005: 100% line coverage gate, measured per test tier, published on every run

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Related:** Extends [ADR-0003](0003-verify-output-against-pinned-bindy-crds.md) and [ADR-0004](0004-single-build-workflow.md)

## Context

forage's failure mode that matters is silent: a record that is not emitted,
or emitted wrong, disappears from DNS at cut-over (threat model T2). The code
is small (two modules, a few hundred lines), so complete coverage is
affordable, and every unexecuted line is a mapping path nobody has seen work.
On 2026-10-05 the suite covered 91.65% of lines: the zone-file parse-error
path, `--zone-dir`, the `directory` option, absolute zone paths, prefix
globs, `--record-types`, unsupported types, absolute record names, name
truncation, YAML output and `--debug` had never run under test.

The Makefile's `test-cov` used cargo-tarpaulin, which does not follow the
binary into the subprocesses `tests/cli_tests.rs` and the e2e suites spawn,
so most of `main.rs` read as untested even when it was.

## Decision

1. Coverage is measured with **cargo-llvm-cov** (LLVM source-based
   instrumentation). It counts code run in the spawned `forage` binary, so
   unit tests, CLI integration tests and e2e runs are all measured.
2. Two tiers, each reported separately so a gap is attributable:
   - **unit + integration** (`make coverage`): `cargo test` with
     instrumentation. **Gate: zero unexecuted lines and 100% of functions**
     (`cargo llvm-cov report --fail-uncovered-lines 0 --fail-under-functions 100`).
     Region coverage is reported, not gated: regions include the `?` error
     arms on serializing plain structs, which cannot fail.
   - **e2e** (`make e2e-coverage`): an instrumented binary run through the
     `schema` and `apply` suites. Reported, not gated: e2e proves the
     contract on the happy path; error paths are the job of the tier above.
3. Every run **publishes** its report where the run is viewed:
   - the job summary (`$GITHUB_STEP_SUMMARY`) gets a per-file table (lines,
     functions, regions) and the totals;
   - the HTML report and `lcov.info` are uploaded as artifacts
     (`coverage-unit-integration`, `coverage-e2e`).
   The unit/integration job (`Coverage - Unit + Integration`) runs in
   `build.yaml` on every event and is part of `PR Checks Passed`; the e2e
   coverage job (`Coverage - E2E`) runs in `e2e.yaml`, which `build.yaml`
   calls, and is part of the E2E gate.
4. The line gate uses llvm-cov's per-line data, not the summary
   percentage. On 2026-10-05 the per-line data showed no unexecuted line in
   `src/` while the summary still reported 99.77%: the summary aggregates
   per-instantiation line counts (398 lines for `mapper.rs` against 385
   mapped lines), and `--fail-under-lines 100` fails on that artifact. The
   published report shows both numbers.
5. Code that genuinely cannot be exercised (for example a `?` on serializing
   a struct that always serializes) is not excluded with markers: it is either
   restructured so it is reachable (as `non_local_zone_type` was on
   2026-10-05), or it is a `?` arm, which only regions count.
6. The shipped binary is never the instrumented one: e2e suites keep running
   the release artifact (ADR-0004 build-once); the instrumented build exists
   only inside the e2e coverage job.

## Consequences

- A PR that adds an untested line fails `PR Checks Passed`. New code arrives
  with its tests, as `rules/testing.md` already demands.
- `cargo-tarpaulin` is dropped; `make test-cov` now means cargo-llvm-cov.
- The e2e coverage job builds forage a second time (instrumented). That is
  the cost of measuring e2e honestly without shipping an instrumented binary.
- Line coverage is not correctness. The fixture tests and the e2e contract
  check (ADR-0003) remain the evidence that output is right; coverage only
  proves no path went unexercised.
