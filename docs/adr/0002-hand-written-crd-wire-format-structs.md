<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0002: Hand-written structs mirror the bindy CRD wire format

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Amended:** 2026-10-05 (rendering moved from serde derives to an explicit `ToJson` impl per struct, ADR-0006; the field-name contract is unchanged)
- **Related:** [ADR-0003](0003-verify-output-against-pinned-bindy-crds.md) (how the mirror is kept honest); [Roadmap 00](../../.github/community/00-bring-your-own-bind9.md) Open Question 2

## Context

forage emits `DNSZone`, `ARecord`, `AAAARecord`, `CNAMERecord`, `MXRecord`,
`TXTRecord`, `SRVRecord` and `CAARecord` resources
(`bindy.firestoned.io/v1beta1`). Roadmap 00 listed three ways to produce
them:

- **A.** Copy the relevant spec structs into forage and derive `Serialize`.
- **B.** Depend on a `bindy-types` crate extracted from bindy.
- **C.** Build untyped `serde_json::Value` trees.

bindy now has a leaf `bindy-api` crate (bindy ADR-0009), but it is not
published to crates.io and pulls in `kube`, `k8s-openapi` and `schemars`,
which forage otherwise does not need (ADR-0001: no Kubernetes client). Option
C gives up the compiler's help for no gain.

## Decision

`src/crd.rs` holds **hand-written** serde structs that mirror only the fields
forage sets, with `#[serde(rename_all = "camelCase")]` so the Rust names map
to bindy's JSON field names. They are not generated, and no CRD generation
tool is run on this repository.

The mirror is pinned to a named bindy release (`BINDY_VERSION` in the
Makefile, currently `v0.7.1`). `src/crd_tests.rs` asserts the exact JSON key
set each spec serializes to, so a rename shows up as a failing unit test, and
ADR-0003's e2e suite checks the result against the real CRDs.

## Consequences

- forage stays small: no `kube`, `k8s-openapi` or `schemars` in the tree.
- **Drift is the main risk** of this choice and it is real: when ADR-0003's
  suite first ran (2026-10-05) bindy v0.7.1 rejected forage's `CNAMERecord`
  (`alias` instead of `target`), `MXRecord` (`preference`/`exchange` instead
  of `priority`/`mailServer`) and `TXTRecord` (`value` string instead of
  `text` array). Unit tests had passed because they only checked the structs
  against themselves. Fixed the same day; see the CHANGELOG.
- Bumping `BINDY_VERSION` is a deliberate change: update the structs, the
  wire-format tests and the pin together, and let the e2e gate prove it
  (`verify-crd-sync` skill).
- If `bindy-api` is ever published as a light, client-free crate, revisit
  option B in a new ADR that supersedes this one.
