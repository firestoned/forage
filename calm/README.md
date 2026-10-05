<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# CALM: Architecture as Code

This directory holds the [FINOS **CALM** (Common Architecture Language Model)](https://calm.finos.org/)
description of forage's architecture. CALM documents are machine-readable
JSON (schema **1.2**) and are the **source of truth** for the architecture
diagram: [`docs/src/architecture/calm-forage.md`](../docs/src/architecture/calm-forage.md)
is generated from it, never hand-drawn.

## Models

| File | What it describes |
|------|-------------------|
| `forage.architecture.json` | The forage CLI (its own parsers + mapper), its inputs on the BIND9 host, the manifest stream on stdout, and the path through the operator's `kubectl` to the API server and the bindy CRDs it must satisfy. |

## Working with these files

Everything is driven from the repository `Makefile` (Node.js 20 or newer;
[`@finos/calm-cli`](https://www.npmjs.com/package/@finos/calm-cli) is fetched
on demand via `npx`, pinned by `CALM_CLI_VERSION`):

```bash
make calm-validate     # schema-validate every calm/*.architecture.json (CI gate)
make calm-docs         # regenerate docs/src/architecture/calm-*.md
make calm-docs-check   # fail if the committed pages are stale (CI gate)
```

### Editing workflow

1. Edit the model here as step 2 of ADD (after the ADR, before any code; see
   `.claude/rules/architecture-driven-development.md`).
2. `make calm-validate` must pass.
3. `make calm-docs`, then commit the model **and** the regenerated page.

The Build workflow runs `calm-validate` and `calm-docs-check` on any PR that
touches `calm/`, the generated pages or `scripts/calm-docs.sh`.
