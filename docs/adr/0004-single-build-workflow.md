<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0004: One Build workflow, build once, signed releases with SBOMs and SLSA provenance

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Supersedes:** the `pr.yaml`, `main.yaml` and `release.yaml` workflows from the initial release
- **Related:** [ADR-0003](0003-verify-output-against-pinned-bindy-crds.md); bindy ADR-0010 (the same supply-chain shape for bindy)

## Context

The initial release carried three near-identical workflows (`pr.yaml`,
`main.yaml`, `release.yaml`), each with its own copy of the five-platform
build matrix, and each compiling the dependency graph twice (a `build` job
and a separate `test` job). Any change to the build had to be made three
times, and nothing verified forage's output against bindy (ADR-0003).
bindy and bindcar had already consolidated into a single `build.yaml` gated
by `github.event_name`; forage should follow the same layout so the three
repositories read the same.

## Decision

1. **`.github/workflows/build.yaml`** is the single workflow for pull
   requests, pushes to `main` and published releases. Jobs are gated on
   `github.event_name`.
2. **Build once.** The Linux binaries are compiled exactly once, in the
   `build` job, with `--locked`; tests run inside the x86_64 leg on pull
   requests (same profile and target, so dependencies compile once). The e2e
   suites consume that artifact. macOS and Windows binaries are built on
   release only.
3. **One required status check**, `PR Checks Passed`, aggregates every job,
   so branch protection does not track matrix job names. Do not rename it.
4. **Releases** ship, per platform, a cosign-signed tarball (keyless,
   Sigstore bundle alongside), a CycloneDX SBOM that passes the NTIA
   minimum-elements gate (`scripts/sbom.sh`) and is attested to the tarball,
   and one SLSA Build L3 provenance file whose subjects are every tarball and
   SBOM. `checksums.sha256` covers everything uploaded.
5. Workflows only install tools, set environment and call Makefile targets
   (`rules/github-workflows.md`); `firestoned/github-actions` composite
   actions are used wherever one exists; third-party actions are pinned by
   commit SHA.
6. Companion workflows: `e2e.yaml` (reusable e2e, ADR-0003),
   `dependabot-auto-merge.yaml` (e2e-gated auto-merge of patch/minor bumps),
   `codeql.yml`, `scorecard.yml` and `security-scan.yaml` (daily advisory
   scan, since new RustSec advisories land without any code change).

## Consequences

- One place to change the build. PR, main and release builds cannot drift.
- Release assets change shape: SBOMs are named `forage-<os>-<arch>.cdx.json`
  (previously `<artifact>-sbom.json`, Linux only) and now exist for all five
  platforms. Anyone scripting downloads by the old names must update.
- The `PR Checks Passed` context must be configured as the required check on
  the `main` ruleset (with `Verify Signed Commits`); this is a repository
  setting, not code.
- No container image is built: forage is a CLI (ADR-0001). If an image is
  ever wanted, it is added to this workflow, from the same build artifact.
