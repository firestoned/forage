<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# forage Roadmap Index

This directory holds forage's roadmap documents. Each one describes a body of
work: *what* and *why*, with a task list and a definition of done.
[`../../ROADMAPS.md`](../../ROADMAPS.md) is the status board that indexes them
and carries the current completion state.

## Numbering

Numbers are a zero-padded two-digit prefix, **contiguous from `00` with no
gaps** and no thematic banding. They are an ordering, not an identity:
inserting or retiring a roadmap renumbers the run, and every reference to the
moved numbers is fixed in the same commit. Reference a roadmap by its padded
number in prose ("roadmap 01") so the number greps against the filename.

Roadmaps are ordered features → fixes and quality → testing. A new roadmap
takes the number at the end of its section and everything after it shifts up.

## Index

### Features

| # | File | What |
|---|---|---|
| 00 | [`00-bring-your-own-bind9.md`](00-bring-your-own-bind9.md) | The original BYOB9 design: one-time import (Phase 1) and live sync (Phase 2) |

### Fixes and quality

| # | File | What |
|---|---|---|
| 01 | [`01-import-fidelity.md`](01-import-fidelity.md) | Every source record reaches the output or is reported |
| 02 | [`02-code-quality-baseline.md`](02-code-quality-baseline.md) | Pedantic clippy, magic numbers, test layout |

### Testing

| # | File | What |
|---|---|---|
| 03 | [`03-e2e-operator-lifecycle.md`](03-e2e-operator-lifecycle.md) | e2e with a running bindy operator and a `dig` round-trip |
