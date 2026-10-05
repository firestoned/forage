<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0006: Own the code we use when it is 500 lines or less; zero third-party crates

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Supersedes:** the dependency choices of the initial release; ADR-0002's "no `kube`, `k8s-openapi` or `schemars`" generalizes to every crate
- **Related:** [ADR-0001](0001-stateless-stdout-cli.md), [ADR-0002](0002-hand-written-crd-wire-format-structs.md), [ADR-0004](0004-single-build-workflow.md), [ADR-0005](0005-coverage-gate-and-published-reports.md); rule `.claude/rules/dependencies.md`

## Context

forage had eight runtime crates and one dev crate. Together with their own
dependencies that is about 160,000 lines of Rust to track, audit and upgrade
(Dependabot PRs, RustSec advisories such as RUSTSEC-2026-0190 in `anyhow`,
deprecations such as `serde_yaml`), to make one binary that reads two text
formats and prints two text formats.

The cost of waiting on upstream became concrete on 2026-10-05: hornet-bind9
0.1 dropped the record after a quoted TXT or a one-line SOA and read an
inherited owner as `IN`. The fix existed upstream but was unreleased, so forage
could only pin the bugs with ignored tests and wait.

What forage actually uses from each crate, measured on 2026-10-05:

| Crate (and what it pulls in) | Upstream lines | What forage uses |
|---|---:|---|
| `hornet-bind9` (+ `winnow`) | 8,699 (parsers 2,667) | Find `zone`, `options { directory }` and `include` in named.conf; parse ten record types from zone files |
| `clap` (+ `clap_builder`, `clap_derive`) | ~33,000 | Nine flags, `--help`, `--version`, one value list |
| `serde`, `serde_json`, `serde_yaml` (+ `unsafe-libyaml`) | ~59,000 | Print fixed manifest structs as pretty JSON and block YAML |
| `anyhow` | 3,919 | A `Result` alias and `.context()` |
| `tracing`, `tracing-subscriber` | ~36,000 | Leveled lines on stderr, chosen by `--debug` or `RUST_LOG` |
| `tempfile` (dev) | 3,183 | One temporary directory per test |

Each used slice fits in 500 lines of our own code, as measured after the
replacement (production Rust, excluding tests, blank lines and comments):

| Replaced | Owned module(s) | Lines |
|---|---|---:|
| hornet-bind9 | `named_conf.rs` + `zone_file.rs` | 492 |
| serde, serde_json, serde_yaml | `json.rs` + `yaml.rs` | 378 |
| clap | `cli.rs` | 144 |
| tracing, tracing-subscriber | `log.rs` | 125 |
| anyhow | `error.rs` | 32 |
| tempfile (dev) | `test_support.rs`, `tests/common/` | 34 + 30 |

About 1,170 lines of production code replace about 160,000 lines of
dependencies. The hornet replacement first came to 552 lines; the budget
forced two simplifications (zone types as normalized strings, one
read-until helper in the named.conf tokenizer) rather than an exception.

## Decision

1. **Rule:** before adding a dependency, and whenever one is touched, measure
   the code forage would need to own to replace *what it uses* (not the whole
   crate). If that is **500 lines or less** of production Rust, write it in
   this repository, test it to the ADR-0005 gate, and drop the crate. Above
   500 lines, the crate may stay, with the measurement recorded in an ADR.
   This applies to dev-dependencies too. The rule lives in
   `.claude/rules/dependencies.md`.
2. Applied now, every crate goes. Replacements, each a module in `src/`:
   - `named_conf.rs`, `zone_file.rs`: parsers for exactly the constructs
     forage maps (replacing hornet-bind9). Zone-file parsing follows RFC 1035
     master-file rules: logical lines joined across parentheses, quotes
     respected, `;` comments, owner inheritance, TTL units. Lines that do not
     parse are reported, not silently dropped.
   - `cli.rs`: argument parsing (replacing clap). Exit code 2 on usage errors,
     as clap had.
   - `json.rs`, `yaml.rs`: a `Value` tree with sorted keys, a pretty JSON
     writer and a block YAML writer (replacing serde, serde_json, serde_yaml).
     Output is byte-identical to the serde-based output for the committed
     fixtures, proven by golden files captured before the switch. String
     quoting and escaping were also differential-tested against the real
     crates on 2026-10-05: 1,000,096 strings through `serde_yaml` and
     1,000,000 through `serde_json`, zero mismatches. The one known
     difference: a string containing a newline is written double-quoted
     instead of as a literal block (DNS data has none).
   - Value inspection (`get`, indexing, comparisons) and `null` exist only
     under `#[cfg(test)]`: production builds values and prints them.
   - `error.rs`: an error type with context (replacing anyhow).
   - `log.rs`: leveled stderr logging (replacing tracing). No ANSI colour.
   - test helpers for temporary directories and JSON parsing in tests
     (replacing tempfile and the test use of serde_json/serde_yaml).
3. Ownership comes with the ADR-0005 gate: every owned line is executed by
   tests. Parsers are additionally checked end to end by the e2e suites
   (ADR-0003) against bindy's CRDs.

## Consequences

- `Cargo.lock` holds forage alone. No Dependabot cargo PRs, no RustSec
  exposure from third-party code, no waiting on upstream releases. The
  supply-chain controls (cargo-deny, cargo-audit, SBOM) stay: they now
  guard against a dependency being reintroduced.
- forage owns about 1,170 more lines of production code, with their tests.
  The release binary is 767 KB.
  Parser fixes ship in forage's own release.
- The three hornet-bind9 0.1 defects (roadmap 01) are fixed by construction;
  their regression tests are un-ignored.
- Log lines lose ANSI colour and change format slightly (same level names,
  same messages). Nothing parses them; the README documents stderr as
  human-oriented.
- hornet-bind9 remains a fine library for tools that need its full surface
  (writing, validating, the whole named.conf grammar). forage did not.
- The rule is a default, not a ban: crypto, TLS, compression or anything
  where 500 lines would be a dangerous reimplementation is recorded in an ADR
  and kept.
