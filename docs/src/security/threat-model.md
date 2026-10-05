<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# Threat Model: forage

**Version:** 1.2
**Last Updated:** 2026-10-05
**Owner:** Erick Bourgeois

> Last full pass 2026-10-05, against ADR-0001 … ADR-0006.
>
> **Revision note (v1.2):** Pass for ADR-0006 (no third-party crates). The
> parser, serializers, argument handling and logging are now forage's own
> code: T4's third-party exposure is gone from the shipped binary (the SBOM
> lists zero packages), and the risk moves to defects in owned code, covered
> by the new T7 and mitigations M-17 (golden byte-identity and differential
> testing against the replaced crates) and M-18 (unparseable lines reported).
> The three hornet-bind9 0.1 record-loss defects in T2 are fixed and their
> tests un-ignored; R1 narrowed accordingly. Components table updated. Assets,
> actors, trust boundaries and the other STRIDE rows re-walked unchanged.
>
> **Revision note (v1.1):** Pass for ADR-0005 (coverage gate) and the
> extended e2e suites (ADR-0003 as amended). Writing tests for full coverage
> surfaced five more silent-integrity defects, all now pinned by ignored
> regression tests: records after a quoted TXT or a one-line SOA dropped, an
> inherited owner imported as `IN` (hornet-bind9 0.1), `include` statements
> not followed, relative targets qualified at the root, and unstable or
> colliding CR names. T2 rewritten to list them; new threat T6 (stale
> orphaned objects after re-import); new mitigations M-15 (coverage gate) and
> M-16 (round-trip and update e2e); accepted risk R1 widened. Assets, actors,
> trust boundaries and the remaining STRIDE rows re-walked unchanged.
>
> **Revision note (v1.0):** First threat model, written when forage adopted
> ADD. Covers the CLI as built (ADR-0001, ADR-0002), the e2e contract check
> (ADR-0003) and the release supply chain (ADR-0004).

## Overview

forage reads a BIND9 `named.conf` and the zone files it references, and
prints bindy CRD manifests to stdout (ADR-0001). It is a one-shot,
offline process: no network connections, no credentials, no state. The
operator applies the output with their own `kubectl` and RBAC.

### Scope

In scope: the `forage` binary, its inputs and output, and how it is built and
released. Out of scope: the bindy operator and BIND9 itself (see bindy's
threat model), and the operator's Kubernetes credentials, which forage never
handles.

The architecture is modeled in
[`calm/forage.architecture.json`](../../../calm/forage.architecture.json),
rendered in [`../architecture/calm-forage.md`](../architecture/calm-forage.md).

## Components

| Component | Role |
|---|---|
| forage CLI | Parses input, maps it, prints manifests (`src/main.rs`) |
| Parsers | `src/named_conf.rs`, `src/zone_file.rs`: forage's own readers (ADR-0006) |
| Writers | `src/json.rs`, `src/yaml.rs`: forage's own JSON/YAML output (ADR-0006) |
| Mapper + CRD structs | `src/mapper.rs`, `src/crd.rs` (ADR-0002) |
| Release pipeline | `.github/workflows/build.yaml` (ADR-0004) |

## Assets

| Asset | Why it matters |
|---|---|
| **A1. Zone data** in the input | May be internal DNS (hostnames, addresses) that should not leave the operator's control |
| **A2. Integrity of the emitted manifests** | Whatever forage prints becomes authoritative DNS once applied; a wrong or missing record is an outage or a hijack |
| **A3. TSIG/RNDC key material** in `named.conf` | `named.conf` often includes `key` blocks; forage must never copy them into output |
| **A4. Integrity of the released binary** | Operators run it on DNS servers that hold A1 and A3 |

## Actors

| Actor | Trust |
|---|---|
| Platform operator running forage | Trusted; supplies input and applies output |
| Author of the input files | Usually the same team; may be a legacy system or an untrusted copy |
| Dependency and action authors | Partially trusted; pinned and scanned |
| Network attacker | Untrusted; forage opens no sockets, so only reaches it through the supply chain |

## Trust boundaries

1. **Input files → forage.** Text from disk crosses into a parser. Input is
   treated as untrusted.
2. **forage → stdout → kubectl → API server.** forage's output is data for
   another tool. The API server's admission (bindy CRD schema) is the final
   validator; the operator is the human gate.
3. **Source → release artifact.** GitHub Actions builds, signs and attests
   the binary.

## STRIDE analysis

### Spoofing

| ID | Threat | Mitigation |
|---|---|---|
| S1 | A tampered or impostor `forage` binary is run on a DNS host | M-1 cosign keyless signatures, M-2 SLSA L3 provenance, M-3 checksums; signed commits enforced in CI |

### Tampering

| ID | Threat | Mitigation |
|---|---|---|
| T1 | Crafted input makes forage emit records the operator did not intend (for example a name that collides with another zone's CR) | M-4 output is reviewed before apply (ADR-0001); M-5 every object is labelled `managed-by=forage` and `zone=<zone>`; CR names are derived from zone and record name |
| T2 | **Silent data loss or corruption**: a record in the input is not emitted, or is emitted wrong, so applying the output removes or changes it in DNS after cut-over | M-6, M-15, M-16. M-18. **Known open gaps** (roadmap 01, each pinned by an ignored test): zones in `include`d files are not imported at all; relative CNAME/MX/SRV/NS targets are qualified at the root (`www.` instead of `www.example.com.`); truncated CR names can be invalid or collide. See accepted risk R1 |
| T3 | forage output drifts from bindy's schema, so fields are rejected or silently dropped | M-7 strict server-side validation against pinned bindy CRDs in CI (ADR-0003); M-8 wire-format unit tests (ADR-0002) |
| T4 | Malicious or vulnerable dependency | Largely removed by ADR-0006: the binary links no third-party crate (SBOM: zero packages); reintroduction is guarded by the dependencies rule, cargo-machete and cargo-deny. Residual: the Rust toolchain and std, and CI actions (T5). Previously: | M-9 `--locked` builds, cargo-deny (advisories, licenses, sources), daily cargo-audit, Dependabot with e2e-gated merges |
| T6 | **Stale orphans after re-import**: CR names are index-based, so adding a record renames later ones; the re-import creates new objects and the old ones keep serving old data until deleted by hand | Partially mitigated: the e2e `update` suite proves a change that does not reorder names touches only what changed (M-16). Open: stable names need an ADR (roadmap 01); until then re-import into a clean namespace or delete by the `managed-by=forage` label first |
| T7 | **Defects in owned code**: forage's own parser or writers misread input or emit wrong YAML/JSON, where a widely used crate might not have | M-15, M-17, M-18, plus the e2e contract check (M-7): every emitted manifest is validated by a real API server |
| T5 | A compromised third-party GitHub Action alters the release | M-10 actions pinned by commit SHA; least-privilege workflow permissions; OpenSSF Scorecard |

### Repudiation

| ID | Threat | Mitigation |
|---|---|---|
| R-1 | Unclear who applied an import | Out of scope for forage: apply happens through the operator's kubectl and is recorded in the Kubernetes audit log. forage logs what it read and emitted on stderr |

### Information disclosure

| ID | Threat | Mitigation |
|---|---|---|
| I1 | TSIG/RNDC secrets from `named.conf` copied into manifests (A3) | M-11 the mapper only reads `zone` statements and zone-file records; `key` blocks are never mapped. No field in `src/crd.rs` can carry key material |
| I2 | Zone data (A1) leaves the host | M-12 forage makes no network connections (ADR-0001); output goes only to stdout |
| I3 | Debug logs (`--debug`) echo record data to stderr | Accepted: stderr is the operator's own terminal. Do not ship debug logs to shared log stores |

### Denial of service

| ID | Threat | Mitigation |
|---|---|---|
| D1 | Huge or pathological input (very large zones) exhausts memory or never finishes | Partially mitigated: forage is a short-lived process the operator can kill and nothing else depends on; parsing is linear in input size. See accepted risk R2 |

### Elevation of privilege

| ID | Threat | Mitigation |
|---|---|---|
| E1 | forage needs or obtains cluster privileges | M-13 forage has no Kubernetes client and no credentials (ADR-0001); privileges stay with the operator's kubectl |
| E2 | Zone-file paths in `named.conf` read files outside the BIND9 tree | Low impact: forage runs as the operator, reads only what that user can read, and prints only records it could parse. Run as an unprivileged user against a copy of the config |

## Mitigations

| ID | Control | Where |
|---|---|---|
| M-1 | Cosign keyless signature per release tarball | `build.yaml` `sign-artifacts` |
| M-2 | SLSA Build L3 provenance over tarballs and SBOMs | `build.yaml` `slsa-provenance` |
| M-3 | `checksums.sha256` over every release asset | `scripts/release.sh` |
| M-4 | Output to stdout only; operator applies | ADR-0001, `src/main.rs` |
| M-5 | Ownership labels on every object | `src/crd.rs` constants, `src/mapper.rs` |
| M-6 | Fixture CLI tests, e2e object counts | `tests/cli_tests.rs`, `tests/e2e/forage-e2e.sh` |
| M-7 | Strict server-side validation against bindy CRDs | ADR-0003, `make e2e-schema` |
| M-8 | Wire-format key-set tests | `src/crd_tests.rs` |
| M-9 | Locked builds, cargo-deny, cargo-audit, Dependabot | `build.yaml`, `security-scan.yaml`, `.cargo/deny.toml` |
| M-10 | SHA-pinned actions, least-privilege permissions, Scorecard | `.github/workflows/` |
| M-11 | Only zone statements and records are mapped | `src/mapper.rs` |
| M-12 | No network I/O | ADR-0001; no network crates in `Cargo.toml` |
| M-13 | No Kubernetes client or credentials | ADR-0001 |
| M-14 | CycloneDX SBOM per binary, NTIA-gated and attested | `build.yaml` `sbom`, `scripts/sbom.sh` |
| M-15 | Every line and function executed by unit + integration tests, gated on every PR; e2e coverage published per run | ADR-0005, `make coverage`, `make e2e-coverage` |
| M-17 | Output byte-identical to the replaced serde implementation (golden files); string quoting/escaping differential-tested against `serde_yaml` and `serde_json` (2,000,096 strings, zero mismatches) | ADR-0006, `tests/fixtures/*/expected.*` |
| M-18 | Every zone-file line that does not parse is reported on stderr with file and line | `src/zone_file.rs`, `src/mapper.rs` |
| M-16 | e2e checks that stored specs equal emitted specs, zone selectors are exact, re-import touches only what changed, and a broken input exits non-zero with empty stdout | ADR-0003 (amended), `tests/e2e/forage-e2e.sh` |

## Accepted risks

| ID | Risk | Revisit when |
|---|---|---|
| R1 | Record loss or corruption in the cases still listed under T2 and T6 (`include` not followed, root-qualified relative targets, unstable CR names), plus zones inside `view` blocks (skipped with only a log line). Operators must compare record counts and spot-check targets before cut-over | Roadmap 01 lands (every ignored regression test is un-ignored and green) |
| R2 | No input size limits (forage reads whole files into memory; includes are not followed, so there is no include depth to bound) | An input-limits option is added, or include following lands (roadmap 01) |
| R3 | `--debug` writes record data to stderr | forage gains a mode that runs unattended (roadmap 00 Phase 2) |
