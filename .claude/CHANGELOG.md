<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Changelog

## [2026-10-05 15:30] - README badges; MSRV verified; hornet 0.2.0 noted

**Author:** Erick Bourgeois

### Changed
- `README.md`: badge sections in bindy's layout (project status, CI/CD, code quality, technology, security, community). Kept only badges backed by something in this repo: Build and E2E workflows, CodeQL, Security Scan, OpenSSF Scorecard, the ADR-0005 coverage gate, ADD/CALM, Rust 1.74+, zero dependencies (ADR-0006), bindy v0.7.1 CRDs, the five release platforms, SPDX, SLSA Build L3, Cosign, signed commits, CycloneDX SBOM, threat model. Omitted bindy's Docker, Trivy, Kubernetes-version, codecov and regulatory-compliance badges: forage ships no image, never talks to a cluster, publishes coverage in its own workflow, and has no compliance documents.
- MSRV `rust-version = 1.74` verified: `cargo +1.74 build --locked` and the full test suite pass.
- `docs/adr/0006-own-small-dependencies.md`, roadmap 01: hornet-bind9 0.2.0 has since been released; the rule still keeps forage's own 492-line parser.

### Why
Match bindy's README; badges must state verifiable facts.

### Impact
- [ ] Breaking change
- [ ] Config change only
- [x] Documentation only

## [2026-10-05 15:00] - Zero third-party crates: own every dependency of 500 lines or less

**Author:** Erick Bourgeois

### Changed
- `docs/adr/0006-own-small-dependencies.md` (new) and `.claude/rules/dependencies.md` (new): before adding or touching a dependency, measure the code forage would own to replace what it uses; 500 lines or less means own it. ADR-0002 amended (rendering via `ToJson`).
- `Cargo.toml`, `Cargo.lock`: removed hornet-bind9, clap, serde, serde_json, serde_yaml, anyhow, tracing, tracing-subscriber and tempfile. The lockfile holds forage alone; the SBOM lists zero packages.
- New modules (measured production lines, excluding tests/blanks/comments): `named_conf.rs` + `zone_file.rs` 492 (hornet-bind9), `json.rs` + `yaml.rs` 378 (serde trio), `cli.rs` 144 (clap), `log.rs` 125 (tracing), `error.rs` 32 (anyhow); `test_support.rs` and `tests/common/` (tempfile; test JSON reader). Each with a `_tests.rs` file; 100% line/function gate holds.
- `src/crd.rs`: serde derives replaced by an explicit `ToJson` impl per struct. `src/mapper.rs`: `map` is infallible (no `?` on serialization); zone-file lines that do not parse are reported as `<file>:<line>: not imported, could not parse: <text>`. `src/main.rs`: `run()`/`render()`, exit 2 on usage errors, 1 on runtime errors.
- Golden files `tests/fixtures/{basic,edge}/expected.{yaml,json}` captured from the serde-based binary before the switch; the new output is byte-identical (unit and CLI tests). YAML quoting and JSON escaping differential-tested against serde_yaml (1,000,096 strings) and serde_json (1,000,000 strings): zero mismatches.
- Fixed by owning the parser: record after a quoted TXT dropped, record after a one-line SOA dropped, inherited owner read as `IN`. Four regression tests un-ignored.
- `.cargo/deny.toml`: the r-efi license exception is gone with the crate.
- CALM: the hornet node is now forage's own parsers; diagram regenerated. Threat model v1.2 (full pass against ADR-0001 … ADR-0006: T4 reduced, new T7 owned-code defects, M-17, M-18). Roadmaps 00/01 and `ROADMAPS.md`, README, CLAUDE.md, `rust-style.md` updated.

### Why
Erick's direction: maintain small code ourselves rather than keep upgrading crates or wait on upstream fixes. About 1,170 owned lines replace about 160,000 lines of dependencies; the hornet-bind9 0.1 record-loss bugs that were waiting on an upstream release are fixed in this repository.

### Impact
- [ ] Breaking change (manifest output byte-identical; stderr log lines lose ANSI colour and sub-second timestamps)
- [ ] Config change only
- [ ] Documentation only

## [2026-10-05 12:30] - Remove unused thiserror dependency

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`, `Cargo.lock`: removed the direct `thiserror` dependency. forage defines no error types (`anyhow` throughout, as a binary should per `rules/rust-style.md`); `cargo-machete` flagged it and failed the PR security job. It remains in the lockfile as a transitive dependency.

### Why
`make cargo-machete` (Build workflow, PR security job) failed on the unused dependency.

### Impact
- [ ] Breaking change
- [ ] Config change only
- [ ] Documentation only

## [2026-10-05 12:00] - Full test coverage with a gate, published reports, extended e2e

**Author:** Erick Bourgeois

### Changed
- `docs/adr/0005-coverage-gate-and-published-reports.md` (new): cargo-llvm-cov, two tiers, gate on zero unexecuted lines + 100% functions, reports published to the run summary and as artifacts. ADR-0003 amended (third suite, variants x fixtures, round-trip checks).
- `src/mapper_tests.rs`: 30 new unit tests driving real named.conf/zone files in temp dirs (SOA/TTL/glue, missing and SOA-less zone files, absolute paths, `--zone-dir`, `directory`, skip-records, cluster-ref, every record type, AAAA collapsing, TXT strings, record-type filter, PTR skip, TTL grouping, absolute owners, CR-name determinism, every `resolve_record_name` branch, zone-type helper).
- `src/mapper.rs`: `is_non_local_zone` → `non_local_zone_type` (returns the matched type) so the warning no longer has an unreachable fallback. Behavior unchanged.
- `tests/cli_tests.rs`: 16 new CLI tests (YAML default, YAML == JSON, missing conf, invalid format, `--debug`, default log level, `RUST_LOG`, `--record-types` normalisation, `--skip-records`, `--zone-filter`, `--cluster-ref`/`--namespace`, `--zone-dir`, labels, `--version`, missing-zone-file warning, unsupported-type debug log).
- 8 `#[ignore]`d regression tests asserting correct behavior for live bugs found while writing them (roadmap 01): records dropped after quoted TXT / one-line SOA and inherited owner read as `IN` (hornet-bind9 0.1; all pass on hornet 0.2), `include` not followed, relative targets qualified at the root, unstable and invalid/colliding CR names. One earlier unit test that asserted the root-qualified target was replaced.
- `tests/fixtures/edge/` (new) and a run-time generated fixture; `tests/e2e/forage-e2e.sh`: schema runs 11 CLI variants x 3 fixtures and a fail-closed check; apply checks stored-spec round-trip, exact zone selectors and JSON re-apply; new `update` suite; `all` mode on one cluster.
- `Makefile`: `coverage`, `coverage-reports`, `coverage-install`, `e2e-coverage`, `e2e-update`; `e2e-all` uses one cluster; tarpaulin targets removed. `scripts/coverage-summary.sh` (new) renders the Markdown table into `$GITHUB_STEP_SUMMARY`.
- `.github/workflows/build.yaml`: `Coverage - Unit + Integration` job (gated, in `PR Checks Passed`). `.github/workflows/e2e.yaml`: `e2e-update` suite and `Coverage - E2E` job (in the E2E gate). Both upload HTML + lcov artifacts.
- Docs: threat model v1.1 (full pass against ADR-0001 … ADR-0005), roadmaps 01/02 and `ROADMAPS.md`, README limitations and coverage, `cargo-quality` and `pre-commit-checklist` skills. CALM unchanged: CI-only change, no runtime component moved.

### Why
Every unexecuted line was a mapping path nobody had seen work, and forage's failure mode is silent. Coverage went from 91.65% to 100% of executed lines and functions (llvm-cov per-line data; the summary percentage reads 99.77% because of an aggregation artifact, see ADR-0005). e2e coverage: 99.77% lines, 100% functions; the one unexecuted line is unreachable through hornet 0.1.

### Impact
- [ ] Breaking change
- [ ] Config change only
- [ ] Documentation only

## [2026-10-05 10:30] - Adopt ADD and bindy's repo layout; relicense to Apache-2.0; fix CNAME/MX/TXT schema drift

**Author:** Erick Bourgeois

### Changed
- `LICENSE`, `NOTICE`, `Cargo.toml`: relicensed from MIT to Apache-2.0. Every file that can hold a comment carries the Apache-2.0 SPDX header; CI license-check now enforces `Apache-2.0`.
- `src/crd.rs`, `src/mapper.rs`: **breaking output fix.** `CNAMERecord.spec.alias` → `target`; `MXRecord.spec.preference`/`exchange` → `priority`/`mailServer`; `TXTRecord.spec.value` (joined string) → `text` (array of the zone file's character-strings). bindy v0.7.1 rejected all three kinds before this.
- `src/crd_tests.rs` (new): wire-format key-set tests for every record spec.
- `src/mapper_test.rs` → `src/mapper_tests.rs`: repo test-file naming; `cargo fmt` applied (the initial release failed `fmt --check`).
- `tests/cli_tests.rs`, `tests/fixtures/` (new): black-box tests of the binary over a fixture covering every mapped type; ignored regression test for the SRV-after-TXT data loss.
- `tests/e2e/forage-e2e.sh` (new), `Makefile`: `e2e-schema` (strict server-side dry run) and `e2e-apply` (apply, count, idempotency, determinism) against bindy v0.7.1 CRDs on kind (`BINDY_VERSION`).
- `.github/workflows/`: `pr.yaml`, `main.yaml`, `release.yaml` replaced by one `build.yaml` (build once, tests in the x86_64 leg, per-platform SBOMs, e2e, `PR Checks Passed` gate, signed + attested + SLSA releases); new `e2e.yaml`, `dependabot-auto-merge.yaml`, `codeql.yml`, `scorecard.yml`, `security-scan.yaml`; `.github/dependabot.yml`, `.github/codeql/codeql-config.yml`. Actions SHA-pinned to the same versions as bindy.
- `Makefile`: `format-check`, `clippy`, `clippy-pedantic`, `test-ci`, `set-version`, `calm-*`, `e2e-*`, `sbom-*`, `release-tarball`, `provenance-subjects`, `release-assets`, `cargo-machete`; `cargo-deny` uses `.cargo/deny.toml` (new); `test-lib` fixed (binary crate has no lib target).
- `scripts/calm-docs.sh`, `scripts/sbom.sh`, `scripts/release.sh` (new): CALM page generation, SBOM NTIA gate, release packaging (moved out of inline workflow bash).
- `docs/adr/0001`–`0004` (new): stateless stdout CLI; hand-written CRD structs; e2e against pinned bindy CRDs; single Build workflow and release supply chain.
- `calm/` (new) + generated `docs/src/architecture/calm-forage.md`; `docs/src/security/threat-model.md` (new, v1.0, full pass against ADR-0001 … ADR-0004).
- `docs/roadmaps/byob9-bring-your-own-bind9.md` → `.github/community/00-bring-your-own-bind9.md` (audited against the tree); new roadmaps 01 (import fidelity), 02 (code-quality baseline), 03 (operator lifecycle e2e); `ROADMAPS.md` status board.
- `.claude/`: ADD, github-workflows and documentation rules; skills split into `.claude/skills/<name>/SKILL.md` (adds `verify-crd-sync`); CLAUDE.md updated.
- `README.md`: usage, mapping, known limitations, release verification.
- `Cargo.lock`: anyhow 1.0.102 → 1.0.104 for RUSTSEC-2026-0190 (unsound `Error::downcast_mut`), found by the first `make cargo-deny` run.

### Why
Bring forage to bindy's Architecture Driven Development standard and repo/workflow layout. The new e2e suite's first run showed bindy rejecting forage's CNAME, MX and TXT output, so the fix ships with it.

### Impact
- [x] Breaking change (emitted CNAME/MX/TXT field names; release SBOM asset names `forage-<os>-<arch>.cdx.json`; license MIT → Apache-2.0)
- [ ] Config change only
- [ ] Documentation only

---
