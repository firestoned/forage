<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 0003: Verify output against a pinned bindy release on a real API server

- **Status:** Accepted
- **Date:** 2026-10-05
- **Deciders:** Erick Bourgeois
- **Amended:** 2026-10-05 (Decision #2 and #3: third `update` suite, schema runs every CLI variant over three fixtures, apply checks stored-spec round-trip and zone selectors; coverage of the suites per ADR-0005)
- **Related:** [ADR-0002](0002-hand-written-crd-wire-format-structs.md); [ADR-0004](0004-single-build-workflow.md)

## Context

forage's whole contract (ADR-0001) is that the manifests it prints are
accepted by bindy. The structs that produce them are a hand-maintained mirror
(ADR-0002). Unit tests can pin what forage serializes, but not whether bindy's
OpenAPI schema, required fields and CEL rules accept it: only an API server
holding the real CRDs can answer that, and `kubectl apply --dry-run=client`
does not consult CRD schemas at all.

## Decision

1. **e2e suites run forage's release binary against a kind cluster holding
   bindy's CRDs**, fetched from `firestoned/bindy` at the tag pinned by
   `BINDY_VERSION`. Only the eight kinds forage emits are installed, so a new
   bindy CRD does not silently widen what the suite claims to cover.
2. Three suites, each owning its cluster so CI runs them in parallel
   (`tests/e2e/forage-e2e.sh`, `make e2e-schema` / `e2e-apply` /
   `e2e-update`; `make e2e-all` runs them on one cluster):
   - **schema**: `kubectl apply --dry-run=server --validate=strict` for every
     CLI variant (default, JSON stream, `--debug`, `--cluster-ref`,
     `--namespace`, `--zone-dir`, `--skip-records`, `--record-types`, suffix,
     prefix and exact `--zone-filter`) over every fixture. Unknown or misspelt
     fields fail. A missing `named.conf` must exit non-zero with empty stdout,
     so a broken run cannot pipe a partial import into `kubectl`.
   - **apply**: apply for real; the number of `managed-by=forage` objects
     equals the number of manifests; every stored spec equals the emitted
     spec field by field (nothing pruned or defaulted away); each DNSZone's
     `recordsFrom` selector matches exactly its own records; re-applying the
     YAML and the JSON stream reports every object `unchanged`; two runs of
     forage emit identical bytes.
   - **update**: import, change the source zone (one address moves, one record
     is added), re-import: exactly one object is reconfigured, one created,
     the rest unchanged.
3. Fixtures: `tests/fixtures/basic/` (every record kind plus a secondary
   zone), `tests/fixtures/edge/` (options block, zone without `file`, missing
   zone file, zone file without SOA or `$TTL`, PTR, absolute owners, a
   relative target, a truncated CR name), and a fixture generated at run time
   for an `options { directory }` base and an absolute zone-file path.
   `tests/cli_tests.rs` pins the same content without a cluster, so most
   regressions fail in `cargo test` before e2e runs.
4. In CI the suites run on every Build (PR, main, release) against the binary
   the `build` job already compiled (ADR-0004), and are part of the
   `PR Checks Passed` gate. `e2e.yaml` is also callable on its own
   (`workflow_dispatch`, Dependabot gate), in which case it builds once itself.
5. The bindy operator itself is **not** installed: the suites check the
   contract (admission), not reconciliation. Running the operator and
   resolving the imported zone is roadmap 03.

## Consequences

- Schema drift between `src/crd.rs` and bindy is caught on the PR that
  introduces it. Its first run caught three wrong field names (ADR-0002).
- CI depends on `raw.githubusercontent.com` serving bindy's tag. A missing
  tag fails the suite loudly; it never passes vacuously.
- Bumping `BINDY_VERSION` is a one-line change whose consequences the gate
  reports; the `verify-crd-sync` skill walks through it.
- Locally the suites need kind, kubectl and a container runtime. On slate
  (rootless podman) kind needs rootful podman:
  `sudo env PATH=$PATH KIND_EXPERIMENTAL_PROVIDER=podman make e2e-all`.
