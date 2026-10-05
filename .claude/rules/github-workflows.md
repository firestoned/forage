<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# GitHub Workflows & CI/CD Standards

## CRITICAL: Never Replace `firestoned/github-actions` With Direct Action Calls

ALL GitHub Actions workflows MUST use composite actions from the `firestoned/github-actions` library. NEVER replace them with direct action calls, even if the underlying action version is outdated.

**Why:** `firestoned/github-actions` is owned by the user (Erick Bourgeois). When an underlying action needs a version bump, fix it in the `firestoned/github-actions` repo: NOT by inlining here.

**Fix process:**
1. Update action version in the `firestoned/github-actions` repository
2. Tag a new release (e.g., v1.3.7)
3. Update the version reference in this repo's workflows

```yaml
# ✅ CORRECT
- name: Cache cargo dependencies
  uses: firestoned/github-actions/rust/cache-cargo@v1.3.6

# ❌ WRONG
- name: Cache cargo dependencies
  uses: actions/cache@v5
```

**Action families:**
- `firestoned/github-actions/rust/cache-cargo`: Cargo dependency caching
- `firestoned/github-actions/rust/setup-rust-build`: Linux cross-compilation setup
- `firestoned/github-actions/rust/build-binary`: Binary compilation
- `firestoned/github-actions/rust/generate-sbom`: SBOM generation
- `firestoned/github-actions/rust/security-scan`: Cargo audit
- `firestoned/github-actions/security/license-check`: SPDX header verification
- `firestoned/github-actions/security/verify-signed-commits`: Commit signature verification
- `firestoned/github-actions/security/cosign-sign`: Keyless signing
- `firestoned/github-actions/versioning/extract-version`: Release version extraction

---

## CRITICAL: All Workflows Must Be Makefile-Driven

Workflows MUST only: install tools, set env vars, and call Makefile targets. All business logic lives in the Makefile.

```yaml
# ✅ GOOD
- name: Run the schema e2e suite
  env:
    FORAGE_BIN: dist/forage
  run: make e2e-schema

# ❌ BAD
- name: Create cluster
  run: |
    kind create cluster
    kubectl apply -f https://raw.githubusercontent.com/...
    # ... 50+ lines of bash ...
```

**Rules:**
- No multi-line bash scripts (except simple tool setup)
- All `run:` commands MUST call Makefile targets (e.g., `make cargo-deny` not `cargo deny check`)
- Makefile targets MUST work identically locally and in CI
- Document targets with `## comments` for `make help`

**e2e targets** (ADR-0003):
- `make e2e-schema`: strict server-side dry run against pinned bindy CRDs
- `make e2e-apply`: apply, count, idempotency, determinism
- `make e2e-all`: both, sequentially, for local use

---

## CRITICAL: Workflows Must Be Reusable and Composable

New workflows MUST support both `workflow_call` (called by other workflows) and standalone triggers.

**Reusable workflow pattern** (see `e2e.yaml`):
```yaml
on:
  workflow_call:
    inputs:
      binary-artifact:
        required: false
        type: string
        default: ''
  workflow_dispatch: {}
```

**Calling reusable workflows, build once** (see `build.yaml`):
```yaml
jobs:
  build:
    # ... uploads artifact forage-linux-amd64
  e2e:
    needs: [build]
    uses: ./.github/workflows/e2e.yaml
    with:
      binary-artifact: forage-linux-amd64
```

---

## Layout (ADR-0004)

| Workflow | Purpose |
|---|---|
| `build.yaml` | The one Build workflow: PR, push to main, release. Owns the `PR Checks Passed` gate. Never split it back into pr/main/release files |
| `e2e.yaml` | Reusable e2e (ADR-0003); called by `build.yaml` and `dependabot-auto-merge.yaml` |
| `dependabot-auto-merge.yaml` | e2e-gated auto-merge of patch/minor Dependabot PRs |
| `codeql.yml`, `scorecard.yml`, `security-scan.yaml` | Scheduled and PR security analysis |

Third-party actions are pinned by full commit SHA with the tag in a comment.
`firestoned/github-actions` uses the same SHAs as bindy so the repos move
together.

**Checklist before adding a new workflow:**
- [ ] Can this be a job in an existing workflow?
- [ ] Is it reusable via `workflow_call`?
- [ ] Does it duplicate existing logic?
- [ ] Can it be a composite action?
