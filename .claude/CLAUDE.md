<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

@.claude/SKILL.md

# Project Instructions for Claude Code

> forage — CLI tool to import BIND9 named.conf into bindy Kubernetes CRD manifests
> Rust binary: reads named.conf → emits YAML/JSON CRD manifests to stdout

**CRITICAL Coding Patterns** (full details in `rules/`):
- **ADD governs ALL work**: ADR → CALM → TDD: `rules/architecture-driven-development.md`
- **TDD**: Write tests FIRST — `rules/testing.md` + `tdd-workflow` skill
- **After ANY Rust change**: run `cargo-quality` skill (NON-NEGOTIABLE)
- **Early returns / magic numbers / style**: `rules/rust-style.md`
- **No third-party crates**: own anything ≤ 500 lines (`rules/dependencies.md`, ADR-0006)

---

## 🚨 CRITICAL: ADD: Architecture Driven Development

**ADD is the governing methodology for this repo**, as for bindy. Every
architecturally significant change follows the fixed pipeline, each step
complete before the next starts:

```
ADR  →  CALM  →  TDD  →  implement  →  docs  →  threat model
```

1. **ADR**: record the decision in `docs/adr/NNNN-title.md` (metadata
   bullets, then Context / Decision / Consequences)
2. **CALM**: update `calm/forage.architecture.json`; `make calm-validate` +
   `make calm-docs` before any implementation
3. **TDD**: tests first, then minimum implementation (`tdd-workflow` skill)
4. **Docs**: CHANGELOG, README, roadmap detail doc + `ROADMAPS.md`
5. **Threat model**: full pass over `docs/src/security/threat-model.md`; bump
   the header stamp. An ADR is not implemented until this pass is done.

Full rule, applicability criteria, and checklist:
`rules/architecture-driven-development.md`.

---

## 🚨 CRITICAL: Output Contract With bindy

forage's only job is output bindy accepts (ADR-0001). The structs in
`src/crd.rs` are a hand-written mirror pinned to `BINDY_VERSION` in the
`Makefile` (ADR-0002), checked by `src/crd_tests.rs` and by the e2e suites
against bindy's real CRDs on a kind cluster (ADR-0003). After any change to
what forage prints: `make e2e-schema` (`verify-crd-sync` skill).

---

## 🚨 CRITICAL: Always Review Official Documentation

When unsure of a decision, ALWAYS read official docs before implementing. Never take shortcuts based on assumptions. Research first, implement second.

---

## 🔍 MANDATORY: Use ripgrep

ALWAYS use `rg` for code search. NEVER use `grep`, `find`, or `lsof`.

- Rust files: `rg -trs "pattern" . -g '!target/'`

---

## 🦀 Rust Workflow

Full style guide: `rules/rust-style.md`. Full testing standards: `rules/testing.md`.

**After ANY `.rs` change:** run `cargo-quality` skill (`cargo fmt` + `cargo clippy` + `cargo test`). Task is NOT complete until all three pass.

### TDD (mandatory)

Write failing tests FIRST, then implement minimum code to pass. See `tdd-workflow` skill.

Test file pattern: `src/foo.rs` → `#[cfg(test)] mod foo_tests;` at bottom → `src/foo_tests.rs`

### Dependency Management

Zero third-party crates by policy (ADR-0006). Before adding any dependency,
follow `rules/dependencies.md`: measure what would be used; 500 lines or less
means write it here, test-first, under the coverage gate.

---

## 📁 File Organization

```
src/
├── main.rs            ← entrypoint: run() + render()
├── cli.rs             ← argument parsing
├── named_conf.rs      ← named.conf reader (zone / options / include / view)
├── zone_file.rs       ← RFC 1035 zone-file reader
├── json.rs, yaml.rs   ← Value tree + writers (byte-identical to serde's output)
├── error.rs, log.rs   ← Error/Context, leveled stderr logging
├── crd.rs             ← hand-written structs mirroring bindy CRD wire format (ToJson)
├── crd_tests.rs       ← wire-format key-set tests
├── mapper.rs          ← named.conf → K8s manifest conversion logic
└── mapper_tests.rs    ← mapper tests (tests always in separate _tests.rs files)
tests/
├── cli_tests.rs       ← black-box tests of the binary over tests/fixtures/
├── fixtures/          ← named.conf + zone files (RFC 2606/5737/3849 data only)
└── e2e/forage-e2e.sh  ← kind + bindy CRD suites (make e2e-*)
docs/adr/              ← ADRs (NNNN-title.md, never renumbered)
docs/src/              ← generated CALM page, threat model
calm/                  ← CALM architecture model
.github/community/     ← roadmap detail docs (NN-title.md); ROADMAPS.md indexes them
```

The CRD structs in `src/crd.rs` are **hand-written** to mirror bindy's wire format — they are NOT auto-generated. Never run a CRD generation tool on this project.

---

## 🧪 Testing

See `rules/testing.md` for full standards.

- Every public function MUST have unit tests
- Tests in separate `_tests.rs` files (never embedded in source)
- Run: `cargo-quality` skill. Specific module: `cargo test <module>` (binary crate, no `--lib`). Verbose: `cargo test -- --nocapture`

---

## 📍 Roadmaps Live In-Repo at `.github/community/` + `ROADMAPS.md`

Detail docs in `.github/community/NN-*.md` (lowercase-hyphen, zero-padded
contiguous numbering) and the one-screen status board in `ROADMAPS.md`. Do
NOT create `docs/roadmaps/`, `ROADMAP.md` or `docs/plans/`. Only documents
naming real infrastructure or unremediated security findings stay outside
the repo (global CLAUDE.md policy).

---

## 🔧 GitHub Workflows & CI/CD

See `rules/github-workflows.md`. Key rules:

- One `build.yaml` for PR / main / release (ADR-0004); `e2e.yaml` is reusable
  and gets the binary from `build`. `PR Checks Passed` is the required check:
  never rename it
- **NEVER** replace `firestoned/github-actions` composite actions with direct action calls
- All workflows delegate logic to Makefile targets (no inline bash scripts)
- New workflows support `workflow_call` for reusability

---

## 📜 License

Apache-2.0 (`LICENSE`, `NOTICE`). Every file that can hold a comment starts
with `Copyright (c) 2025 Erick Bourgeois, firestoned` and
`SPDX-License-Identifier: Apache-2.0`; CI's license-check enforces it.

---

## 📝 Documentation Requirements

See `rules/documentation.md`.

- Update `.claude/CHANGELOG.md` with `**Author:**` on EVERY code change (MANDATORY, no exceptions)

---

## 🚫 Things to Avoid

- `unwrap()` in production — use `?` or explicit error handling
- Magic numbers — define named constants
- Hardcoded paths — use CLI args or constants
- `sleep()` for synchronization

---

## 💡 Helpful Commands

```bash
cargo run -- --conf /etc/bind/named.conf          # Run locally
cargo run -- --conf /etc/bind/named.conf --debug   # Debug logging
cargo test                                          # Run all tests
make e2e-all                                        # kind + bindy CRD suites
make calm-validate calm-docs                        # architecture model
```

Skills (`.claude/skills/`): `cargo-quality`, `tdd-workflow`, `verify-crd-sync`, `pre-commit-checklist`, `update-changelog`.

---

## 📋 PR/Commit Checklist

**Run `pre-commit-checklist` skill before EVERY commit. A task is NOT complete until it passes.**

---

## 🔗 Project References

- [hornet-bind9](https://github.com/firestoned/hornet): BIND9 parser library forage used until ADR-0006
- [bindy](https://github.com/firestoned/bindy) — operator whose CRD schema we mirror in `src/crd.rs`
