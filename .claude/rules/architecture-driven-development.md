<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Architecture Driven Development (ADD)

> **ADD is the governing methodology for forage**, as it is for bindy.
> Architecture is designed, recorded, and visualized **before** code is
> written, and its security posture is re-verified **after**. ADRs, the CALM
> model and the threat model are first-class deliverables, equal in importance
> to the code and the tests.

ADD layers *on top of* the TDD discipline (`rules/testing.md`); it does not
replace it. The order is fixed:

```
ADR  →  CALM  →  TDD  →  implement  →  docs  →  threat model
```

## The ADD cycle

For any **architecturally significant** change, complete each step before
starting the next:

### 1. ADR: decide and record (FIRST)

Write or update an Architecture Decision Record in `docs/adr/NNNN-title.md`
(lowercase-hyphen, four-digit zero-padded sequential number, never
renumbered). Title line is `# NNNN: Title`.

**Metadata is a bullet list under the title, never a `## Status` section**,
one field per bullet, so status and date stay greppable:

```markdown
# NNNN: Title

- **Status:** Accepted
- **Date:** 2026-10-05
- **Proposed:** 2026-10-04          (when it sat Proposed first)
- **Deciders:** Erick Bourgeois
- **Amended:** 2026-10-10 (Decision #3, …)
- **Supersedes:** ADR-NNNN …
- **Related:** Extends [ADR-NNNN](…) …
```

`Status` and `Date` are required; the rest appear only when they apply. Then
the standard sections: **Context**, **Decision**, **Consequences**.

Status runs Proposed → Accepted (→ Superseded by NNNN). *Accepted* records
that the decision is made, not that it shipped. One decision per ADR. If a
change reverses an earlier ADR, mark the old one *Superseded* and link
forward.

### 2. CALM: model and visualize

Update the FINOS CALM model (`calm/forage.architecture.json`) to reflect the
decision: nodes, relationships, protocols. Then:

```sh
make calm-validate    # the model conforms to CALM 1.2 (CI gate)
make calm-docs        # regenerate docs/src/architecture/calm-forage.md
make calm-docs-check  # committed page matches the model (CI drift gate)
```

A change that isn't reflected in CALM isn't designed yet. See
`calm/README.md`.

### 3. TDD: red / green / refactor

Only now write code, tests first, per `rules/testing.md` and the
`tdd-workflow` skill. After any `.rs` change, run the `cargo-quality` skill.
If the change touches what forage emits, `make e2e-schema` must also pass
(ADR-0003).

### 4. Docs, including **both** roadmap artefacts

Update `.claude/CHANGELOG.md` (with `**Author:**`), `README.md` and any
affected `docs/src/` page, per `rules/documentation.md`.

**If the work advanced a roadmap item, update both places, in this commit:**

1. the detail doc, `.github/community/NN-*.md`: tick the checkbox or update
   the table row, and say what actually landed;
2. **`ROADMAPS.md`** at the repo root: the status board row.

The detail doc is the task list you work from; `ROADMAPS.md` is the
one-screen answer to "what state is this project in". The trigger is
**completion, not change**: if a checkbox is true now, tick it now, even when
the work that made it true was an earlier session's. While you are in the
detail doc, **audit the rest of it against the tree**.

### 5. Threat model: full pass (LAST)

Once the ADR is implemented, make a **full pass** over
`docs/src/security/threat-model.md`: assets, actors, trust boundaries, STRIDE
tables, mitigations, accepted risks. Map every new or changed threat to a
control that exists in `src/`, `tests/` or `.github/`, or record it as an
accepted risk with a *Revisit when*.

Then bump the header stamp: `**Last Updated:**`, the version, **and** the
`Last full pass YYYY-MM-DD, against ADR-0001 … ADR-NNNN` line. An unchanged
stamp means the pass did not happen. "No change" is a valid conclusion; bump
the stamp and say so in the CHANGELOG.

**An ADR is not implemented until this pass is done.**

## When does ADD apply?

**Full ADR + CALM + threat-model pass** (architecturally significant):

- Anything that changes forage's contract with bindy: new emitted kinds,
  field mappings with judgement in them, bumping `BINDY_VERSION`
- New inputs or outputs: reading new BIND9 constructs (views, includes over
  the network), new output formats or destinations
- Anything that breaks ADR-0001's "offline and stateless": a Kubernetes
  client, network access, credentials, a long-running mode (roadmap 00
  Phase 2)
- Release and supply-chain changes (signing, provenance, SBOM shape)
- Any decision where "why A over B" is worth recording

**TDD only** (no ADR/CALM needed):

- Typos, comment/doc tweaks, formatting
- Isolated bug fixes with no architectural impact (for example a wrong field
  name against an already-pinned bindy schema)
- Mechanical refactors that preserve behavior and structure

> When unsure whether a change is "architectural", **write the ADR.** A
> short, slightly redundant ADR costs little; an undocumented decision costs
> the next person a re-derivation.

## Checklist (paste into the work)

- [ ] ADR written/updated in `docs/adr/NNNN-*.md`: metadata bullets, then
      Context/Decision/Consequences
- [ ] CALM model updated; `make calm-validate`, `make calm-docs`,
      `make calm-docs-check` clean
- [ ] Tests written **first**, then implementation (TDD)
- [ ] `cargo-quality` passes; `make e2e-schema` passes if output changed
- [ ] CHANGELOG + docs updated
- [ ] Roadmap detail doc **and** `ROADMAPS.md` both updated for anything that
      completed (and the rest of the detail doc audited against the tree)
- [ ] Full threat-model pass done; header stamp bumped
