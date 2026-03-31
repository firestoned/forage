@.claude/SKILL.md

# Project Instructions for Claude Code

> forage — CLI tool to import BIND9 named.conf into bindy Kubernetes CRD manifests
> Rust binary: reads named.conf → emits YAML/JSON CRD manifests to stdout

**CRITICAL Coding Patterns** (full details in `rules/`):
- **TDD**: Write tests FIRST — `rules/testing.md` + `tdd-workflow` skill
- **After ANY Rust change**: run `cargo-quality` skill (NON-NEGOTIABLE)
- **Early returns / magic numbers / style**: `rules/rust-style.md`

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

Before adding deps: verify actively maintained (commits in last 6 months), prefer well-known crates, document reason in CHANGELOG.

---

## 📁 File Organization

```
src/
├── main.rs           ← CLI entrypoint (clap)
├── crd.rs            ← serde structs mirroring bindy CRD wire format
├── mapper.rs         ← named.conf → K8s manifest conversion logic
├── mapper_test.rs    ← mapper tests
└── <module>_tests.rs ← tests always in separate files
```

The CRD structs in `src/crd.rs` are **hand-written** to mirror bindy's wire format — they are NOT auto-generated. Never run a CRD generation tool on this project.

---

## 🧪 Testing

See `rules/testing.md` for full standards.

- Every public function MUST have unit tests
- Tests in separate `_tests.rs` files (never embedded in source)
- Run: `cargo-quality` skill. Specific module: `cargo test --lib <module>`. Verbose: `cargo test -- --nocapture`

---

## 📝 Documentation Requirements

- Update `.claude/CHANGELOG.md` with `**Author:**` on EVERY code change (MANDATORY — no exceptions)

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
```

Skills: `cargo-quality`, `tdd-workflow`, `pre-commit-checklist`, `update-changelog`.

---

## 📋 PR/Commit Checklist

**Run `pre-commit-checklist` skill before EVERY commit. A task is NOT complete until it passes.**

---

## 🔗 Project References

- [hornet-bind9](https://github.com/firestoned/hornet) — BIND9 named.conf parser used as input
- [bindy](https://github.com/firestoned/bindy) — operator whose CRD schema we mirror in `src/crd.rs`
- [clap documentation](https://docs.rs/clap/)
