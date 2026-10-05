<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Roadmaps

High-level index of forage's roadmap documents. Full detail for each item
lives in [`.github/community/`](.github/community/); this file tracks what
each one is and its current completion status.

Architecturally significant work in any roadmap below still goes
**ADR → CALM → TDD → implement → docs → threat model**, in that order (see
[`.claude/rules/architecture-driven-development.md`](.claude/rules/architecture-driven-development.md)).
A roadmap entry describes *what* and *why*; it does not skip the ADR for
*how*.

## Status legend

| Symbol | Meaning |
|---|---|
| ✅ | Done: implemented, tested, in the codebase today |
| 🔶 | In progress: some of it exists, not complete |
| ⛔ | Not started |
| 📄 | Reference doc: not a phase with a completion state |

## Index

Statuses were verified against `initial` on 2026-10-05.

### Features

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [00](.github/community/00-bring-your-own-bind9.md) | Bring Your Own BIND9 | 🔶 | Phase 1 (one-time import) done as `forage` ([ADR-0001](docs/adr/0001-stateless-stdout-cli.md)); only `--dry-run` missing (→ 01). Phase 2 (live sync) not started and needs an ADR superseding ADR-0001 |

### Fixes and quality

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [01](.github/community/01-import-fidelity.md) | Import fidelity | 🔶 | Fixed 2026-10-05: CNAME/MX/TXT schema drift; hornet 0.1 record loss/corruption (own parser, ADR-0006); unparseable lines now reported. Open, pinned by ignored tests: **`include` not followed** (zones in included files import nothing), relative targets qualified at the root (`$ORIGIN` not applied), unstable and colliding CR names (ADR). Also open: relative `directory`, `view` zones (ADR), shell SOA (ADR), `--dry-run` |
| [02](.github/community/02-code-quality-baseline.md) | Code-quality baseline | 🔶 | fmt, test-file naming and mapper unit coverage done 2026-10-05 (100% line/function gate, [ADR-0005](docs/adr/0005-coverage-gate-and-published-reports.md)). 14 pedantic clippy findings (incl. six `u32 as i32` wrapping casts), magic numbers, repeated literals |

### Testing

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [03](.github/community/03-e2e-operator-lifecycle.md) | e2e with a running bindy operator | ⛔ | ADR-0003's suites stop at admission; this adds reconcile + `dig` round-trip |

## Numbering

Numbers are a zero-padded two-digit prefix, contiguous from `00` with no gaps
and no thematic banding. They are an ordering, not an identity: inserting or
retiring a roadmap renumbers the run, and every reference to the moved numbers
is fixed in the same commit. Reference a roadmap by its padded number in prose
("roadmap 01") so the number greps against the filename.

## Keeping this current

When a roadmap item's status changes, update its row here in the same commit
that makes the change: this file is a status board, not documentation of
intent. Detailed task-level tracking stays inside each roadmap doc.
