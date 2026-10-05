<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Documentation Standards

## Before Marking Any Task Complete

ALWAYS ask: "Does documentation need to be updated?"

Applies to: code changes, CLI flag changes, output-format changes, CI
changes, architecture changes.

---

## Documentation Update Workflow

1. **Analyze the change**: user-facing impact? architectural implications
   (then ADD applies: `rules/architecture-driven-development.md`)? new flags
   or output?
2. **Update in this order:**
   - `.claude/CHANGELOG.md` (`update-changelog` skill; `**Author:**` is MANDATORY)
   - `README.md` for flags, usage, mapping table, limitations
   - `docs/adr/` if a decision was made or amended
   - `calm/forage.architecture.json` + `make calm-docs` if structure changed
   - `docs/src/security/threat-model.md` after an ADR lands (full pass)
   - `.github/community/NN-*.md` **and** `ROADMAPS.md` if a roadmap item moved
3. **Verify**: read the README as a new user; `cargo doc --no-deps` builds
   without warnings.

---

## What to Update by Change Type

**Mapping changes** (`src/mapper.rs`, `src/crd.rs`):
- README mapping table and examples
- Field names verified against bindy's CRDs at `BINDY_VERSION`
  (`verify-crd-sync` skill), never guessed
- `tests/fixtures/` when a new record type or construct is supported

**CLI changes** (`src/main.rs`):
- README flags table; `--help` text is the source of truth, keep them equal

**CI changes** (`.github/workflows/`, `Makefile`):
- `rules/github-workflows.md` if the layout changed; ADR-0004 if the shape
  of the build or release changed

**Bug fixes**:
- CHANGELOG entry; tick the roadmap 01/02 item if it was tracked there

---

## Examples Must Match bindy's CRDs

ALWAYS verify field names against bindy's CRDs at the pinned
`BINDY_VERSION` (`deploy/operator/crds/*.crd.yaml` in `firestoned/bindy`).
`make e2e-schema` is the mechanical check.

---

## Changelog Requirements

Every entry in `.claude/CHANGELOG.md` MUST have `**Author:**`, no exceptions.
Format: see the `update-changelog` skill.

---

## Code Comments

All public functions and types MUST have rustdoc comments with `# Arguments`,
`# Returns` and `# Errors` where they apply.

---

## Validation Checklist

- [ ] `.claude/CHANGELOG.md` updated with `**Author:**`
- [ ] `README.md` matches `forage --help` and current mapping
- [ ] ADR / CALM / threat model updated if ADD applied
- [ ] Roadmap detail doc and `ROADMAPS.md` updated for completed items
- [ ] `make e2e-schema` passes if output changed
