<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 03: e2e against a running bindy operator

> Prove that an import does not just pass admission but actually serves:
> install bindy, apply forage's output, and resolve every record from the
> fixture with `dig`.

ADR-0003 deliberately stops at the API server: the `schema` and `apply`
suites check the contract (bindy's CRD schema) and idempotency, but nothing
reconciles the objects. A field can be schema-valid and still mean something
different to bindy (for example a relative record name bindy treats as
absolute, or a TXT split differently).

## Tasks

- [ ] New suite `e2e-lifecycle`: install bindy at `BINDY_VERSION` from its
      release `install.yaml` (digest-pinned), create a `Bind9Cluster`, run
      forage with `--cluster-ref`, apply, wait for every `DNSZone` and record
      to report `Ready`.
- [ ] Resolve every record in `tests/fixtures/basic/db.example.com` against
      the BIND9 service and compare with the source zone file (a
      `dig`-based round-trip: source RRset equals served RRset).
- [ ] Secondary-zone shell: assert what bindy does with it, and feed the
      answer back into roadmap 01's shell-SOA item.
- [ ] Add the suite to the `e2e.yaml` matrix and the `PR Checks Passed` gate.
- [ ] Update ADR-0003 (amend: the lifecycle suite) and the CALM model (the
      bindy operator node moves from "consumer" to "exercised in CI").

## Definition of done

A green lifecycle suite in CI, and the round-trip diff empty for the basic
fixture.
