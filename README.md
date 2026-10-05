<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# forage

## Project Status

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![GitHub Release](https://img.shields.io/github/v/release/firestoned/forage)](https://github.com/firestoned/forage/releases/latest)
[![GitHub commits since latest release](https://img.shields.io/github/commits-since/firestoned/forage/latest)](https://github.com/firestoned/forage/commits/main)
[![Last Commit](https://img.shields.io/github/last-commit/firestoned/forage)](https://github.com/firestoned/forage/commits/main)

## CI/CD Status

[![Build](https://github.com/firestoned/forage/actions/workflows/build.yaml/badge.svg)](https://github.com/firestoned/forage/actions/workflows/build.yaml)
[![E2E Tests](https://github.com/firestoned/forage/actions/workflows/e2e.yaml/badge.svg)](https://github.com/firestoned/forage/actions/workflows/e2e.yaml)

## Code Quality

[![Coverage gate](https://img.shields.io/badge/coverage%20gate-100%25%20lines%20%26%20functions-brightgreen)](docs/adr/0005-coverage-gate-and-published-reports.md)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/firestoned/forage/badge)](https://api.securityscorecards.dev/projects/github.com/firestoned/forage)
[![CodeQL](https://github.com/firestoned/forage/actions/workflows/codeql.yml/badge.svg)](https://github.com/firestoned/forage/actions/workflows/codeql.yml)
[![Security Scan](https://github.com/firestoned/forage/actions/workflows/security-scan.yaml/badge.svg)](https://github.com/firestoned/forage/actions/workflows/security-scan.yaml)
[![Architecture](https://img.shields.io/badge/architecture-ADD%20%7C%20CALM-blueviolet)](calm/README.md)

## Technology & Compatibility

[![Rust](https://img.shields.io/badge/rust-1.74+-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Dependencies](https://img.shields.io/badge/dependencies-0-brightgreen)](docs/adr/0006-own-small-dependencies.md)
[![bindy](https://img.shields.io/badge/bindy%20CRDs-v0.7.1-blue)](https://github.com/firestoned/bindy/releases/tag/v0.7.1)
[![BIND9](https://img.shields.io/badge/BIND9-named.conf-blue)](https://www.isc.org/bind/)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](https://www.linux.org/)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)
[![Windows](https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white)](https://www.microsoft.com/windows)

## Security & Compliance

[![SPDX](https://img.shields.io/badge/SPDX-License--Identifier-blue)](https://spdx.dev/)
[![SLSA Build L3](https://img.shields.io/badge/SLSA-Build%20L3-blue)](docs/adr/0004-single-build-workflow.md)
[![Cosign Signed](https://img.shields.io/badge/releases-signed-brightgreen.svg)](#verifying-a-release)
[![Commits Signed](https://img.shields.io/badge/commits-signed-brightgreen.svg)](.github/workflows/build.yaml)
[![SBOM](https://img.shields.io/badge/SBOM-CycloneDX-orange)](https://cyclonedx.org/)
[![Threat Model](https://img.shields.io/badge/threat%20model-v1.2-purple)](docs/src/security/threat-model.md)

## Community & Support

[![Issues](https://img.shields.io/github/issues/firestoned/forage)](https://github.com/firestoned/forage/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/firestoned/forage)](https://github.com/firestoned/forage/pulls)

## Overview

Import an existing BIND9 server into a [bindy](https://github.com/firestoned/bindy)-managed
Kubernetes cluster. forage reads `named.conf`, parses the zone files it
references, and prints `DNSZone` and record CRs as YAML or JSON on stdout.
It is a single static binary with no third-party crates
([ADR-0006](docs/adr/0006-own-small-dependencies.md)).
It never talks to the cluster: you review the output and apply it with your
own `kubectl`.

```bash
forage --conf /etc/bind/named.conf --namespace dns > import.yaml
kubectl apply -f import.yaml
```

## What it emits

| Source | bindy resource |
|---|---|
| `zone` statement + SOA + NS (with A glue) | `DNSZone` (`spec.soaRecord`, `spec.nameServers`, `spec.recordsFrom` selecting its own records) |
| `A` / `AAAA` | `ARecord` / `AAAARecord`; several addresses for one name and TTL collapse into one CR |
| `CNAME`, `MX`, `TXT`, `SRV`, `CAA` | `CNAMERecord`, `MXRecord`, `TXTRecord`, `SRVRecord`, `CAARecord` |
| secondary / stub / forward zones | `DNSZone` shell only (no local zone file) |
| other types (PTR, DNSSEC, …) | skipped, logged at debug level |

Every object is labelled `bindy.firestoned.io/managed-by=forage`; records also
carry `bindy.firestoned.io/zone=<zone>`. Output targets
`bindy.firestoned.io/v1beta1` and is verified in CI against bindy
**v0.7.1**'s CRDs ([ADR-0003](docs/adr/0003-verify-output-against-pinned-bindy-crds.md)).

## Options

```
-c, --conf <CONF>                  Path to named.conf [default: /etc/bind/named.conf]
    --zone-dir <ZONE_DIR>          Base directory for relative zone-file paths
    --namespace <NAMESPACE>        Namespace for emitted resources [default: bindy-system]
    --cluster-ref <CLUSTER_REF>    Value for DNSZone spec.clusterRef
    --zone-filter <ZONE_FILTER>    Glob, e.g. "*.example.com" [default: *]
    --skip-records                 Emit only DNSZone resources
    --record-types <RECORD_TYPES>  e.g. A,AAAA,CNAME,MX,TXT,SRV,CAA (default: all)
    --output <OUTPUT>              yaml | json [default: yaml]
-d, --debug                        Debug logging on stderr
```

## Known limitations

**Compare record counts and spot-check targets between the source and the
output before cutting over.** A zone-file line forage cannot parse is never
dropped silently: it is reported on stderr as
`<file>:<line>: not imported, could not parse: <text>`. Tracked in
[roadmap 01](.github/community/01-import-fidelity.md), each pinned by an
ignored test (`cargo test -- --ignored`):

- zones declared in `include`d files are **not imported**;
- relative CNAME/MX/SRV/NS targets are qualified at the root
  (`www.` instead of `www.example.com.`): write targets fully qualified;
- CR names are index-based: adding a record can rename others, so a
  re-import leaves orphans. Re-import into a clean namespace, or delete by
  `-l bindy.firestoned.io/managed-by=forage` first;
- zones inside `view` blocks are skipped;
- a relative `directory` option resolves against the current directory, not
  `named.conf`'s (use `--zone-dir`);
- shell `DNSZone`s for secondary zones carry a placeholder SOA.

## Verifying a release

Each release ships, per platform, a tarball, its cosign bundle, a CycloneDX
SBOM, one SLSA provenance file and `checksums.sha256`
([ADR-0004](docs/adr/0004-single-build-workflow.md)).

```bash
cosign verify-blob forage-linux-amd64.tar.gz \
  --bundle forage-linux-amd64.tar.gz.bundle \
  --certificate-identity-regexp '^https://github\.com/firestoned/forage/\.github/workflows/build\.yaml@refs/tags/' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
slsa-verifier verify-artifact forage-linux-amd64.tar.gz \
  --provenance-path <version>.intoto.jsonl --source-uri github.com/firestoned/forage
```

## Development

forage follows Architecture Driven Development, like bindy: ADR → CALM → TDD →
docs → threat model.

| Where | What |
|---|---|
| [`docs/adr/`](docs/adr/) | Architecture decisions |
| [`calm/`](calm/) | CALM model; diagram in [`docs/src/architecture/calm-forage.md`](docs/src/architecture/calm-forage.md) |
| [`docs/src/security/threat-model.md`](docs/src/security/threat-model.md) | Threat model |
| [`ROADMAPS.md`](ROADMAPS.md) | Roadmap status board ([details](.github/community/)) |

Every Build run publishes coverage on its summary page: unit + integration
(gated at 100% of lines and functions) from the Build workflow, e2e from the
E2E workflow it calls. The HTML reports are attached as the
`coverage-unit-integration` and `coverage-e2e` artifacts
([ADR-0005](docs/adr/0005-coverage-gate-and-published-reports.md)).

```bash
make test        # unit + CLI fixture tests
make lint        # rustfmt check + clippy
make coverage    # cargo-llvm-cov: 100% line/function gate, HTML in target/coverage/
make e2e-all     # kind + bindy CRDs: schema (variants x fixtures), apply, update
make e2e-coverage  # the e2e suites against an instrumented build
make calm-validate calm-docs-check
make help        # everything else
```

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
