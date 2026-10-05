---
name: verify-crd-sync
description: Check that forage's hand-written CRD structs (src/crd.rs) still match bindy's CRDs at the pinned BINDY_VERSION, or bump the pin. Use after editing src/crd.rs or src/mapper.rs, when bindy releases, or when an import is rejected by the API server.
---

<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# verify-crd-sync

forage mirrors bindy's wire format by hand (ADR-0002). The pin is
`BINDY_VERSION` in the `Makefile` (and the default in
`tests/e2e/forage-e2e.sh`).

## Check the current pin

```bash
# Wire-format unit tests (fast, no cluster)
cargo test crd_tests

# Real API server, strict validation (needs kind + kubectl + a runtime)
make e2e-schema
# on slate (rootless podman cannot run kind):
sudo env PATH=$PATH KIND_EXPERIMENTAL_PROVIDER=podman make e2e-schema
```

An `unknown field "spec.X"` error names the struct field to fix.

## Read a CRD's spec fields directly

```bash
git -C ~/dev/bindy show <tag>:deploy/operator/crds/<kind>s.crd.yaml \
  | python3 -c 'import sys,yaml; d=[x for x in yaml.safe_load_all(sys.stdin) if x][0]; s=d["spec"]["versions"][0]["schema"]["openAPIV3Schema"]["properties"]["spec"]; print(s.get("required")); print(sorted(s["properties"]))'
```

## Bumping BINDY_VERSION (architecturally significant: ADD applies)

1. Diff the eight CRDs forage emits between the old and new tag.
2. Update `src/crd_tests.rs` first (RED), then `src/crd.rs` / `src/mapper.rs`.
3. Bump `BINDY_VERSION` in the `Makefile` and `tests/e2e/forage-e2e.sh`.
4. `make e2e-all` green; amend ADR-0002 with the new pin; CHANGELOG.

## Verification

`cargo test` and `make e2e-schema` both pass at the pinned version.
