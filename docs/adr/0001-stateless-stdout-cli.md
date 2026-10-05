<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0001: forage is a standalone, stateless CLI that prints manifests to stdout

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Related:** [Roadmap 00](../../.github/community/00-bring-your-own-bind9.md) (Phase 1 design, Open Question 1)

## Context

bindy manages DNS declaratively, but a team arriving with an existing BIND9
server has no path from its `named.conf` and zone files to bindy CRs other
than writing them by hand. Roadmap 00 scoped the tool that closes that gap
and left two placement questions open: where the code lives (bindcar
workspace, bindy repository, or its own repository), and whether it talks to
the cluster itself.

The forces:

- The parser (`hornet-bind9`) and the release cadence are independent of
  bindy's operator and of bindcar's sidecar. Coupling forage into either
  repository would tie a one-shot migration tool to their CI, their image
  builds and their version numbers.
- Migration is a reviewed, audited change in a regulated environment. The
  person migrating should see exactly what will be created before anything is
  created, and the creation should happen under that person's own RBAC, not
  a credential the tool holds.
- The BIND9 host and the target cluster are often on different networks. A
  tool that only reads files can run on the BIND9 host (or against a copy of
  its configuration) with no cluster access at all.

## Decision

1. forage lives in its own repository, `firestoned/forage`, as a single
   binary crate.
2. forage is **stateless and offline**: it reads `named.conf` and the zone
   files it references, prints bindy CRD manifests to stdout (YAML, or JSON
   with `--output json`), writes diagnostics to stderr, and exits. It opens
   no network connections, holds no credentials and keeps nothing between
   runs.
3. Applying the output is the operator's job, with their own client:
   `forage --conf named.conf | kubectl apply -f -`, or write to a file,
   review, then apply.
4. Output is deterministic for a given input: `DNSZone` resources first, then
   record CRs, in a stable order, so that two runs diff cleanly and re-applying
   is a no-op.

## Consequences

- forage has no Kubernetes client dependency, no RBAC of its own, and nothing
  to deploy. Its attack surface is "parse untrusted text files and print".
- Every manifest passes through `kubectl` and API-server admission before it
  exists, so the bindy CRD schema is the final validator. forage's job is to
  produce output that validator accepts, which ADR-0003 verifies in CI.
- Determinism is part of the contract and is tested (CLI tests and the e2e
  `apply` suite).
- Rules out, for now, a live-sync mode. Roadmap 00 Phase 2 (watch files,
  reconcile against the cluster) would need a Kubernetes client, credentials
  and a long-running process; it needs its own ADR that amends or supersedes
  this one before any of it is built.
