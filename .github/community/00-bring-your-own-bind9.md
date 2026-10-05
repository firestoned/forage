<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 00: Bring Your Own BIND9 (BYOB9)

> Onboard an existing BIND9 instance into a bindy-managed Kubernetes cluster
> by importing its `named.conf` (and zone files) as native bindy CRD resources.

## Status (audited against the tree 2026-10-05)

- **Phase 1 (one-time import): done**, shipped as `forage` in its own
  repository ([ADR-0001](../../docs/adr/0001-stateless-stdout-cli.md)). The
  binary name `byob9` below is the working title; `forage` was chosen.
- Follow-up fidelity gaps found while auditing (SRV dropped after TXT,
  relative `directory`, `view` zones, `--dry-run`) are tracked in
  [roadmap 01](01-import-fidelity.md), not here.
- **Phase 2 (live sync): not started.** It reverses ADR-0001 (forage would
  need a Kubernetes client and credentials), so it starts with a new ADR.

This document is kept as the original design record; the milestone tables
below carry the current state.

---

## Problem Statement

Users running BIND9 on bare-metal or VMs before adopting Kubernetes face a
migration barrier: bindy can manage DNS declaratively, but it starts from
scratch. There is no path for importing the zones and records that already
exist in a live `named.conf` / zone-file set into the cluster as
`DNSZone`, `ARecord`, `AAAARecord`, `CNAMERecord`, `MXRecord`, `TXTRecord`,
`NSRecord`, `SRVRecord`, and `CAARecord` custom resources.

The bindcar `drone` mode (out-of-cluster) closes the *management* gap — bindy
can now drive a remote BIND9. But the *onboarding* gap remains: turning an
existing BIND9 configuration into bindy CRs has to be done by hand today.

**BYOB9** fills that gap. It reads a `named.conf` (and optionally the zone
files it references), maps the configuration to bindy CRD manifests, and
emits them to stdout so the operator can pipe directly into the cluster:

```bash
byob9 --conf /etc/bind/named.conf | kubectl apply -f -
```

---

## Position in the Ecosystem

```
                ┌──────────────────────────────────────────┐
                │          Queen-Bee Kube Cluster           │
                │                                           │
                │  ┌─────────┐  ┌──────────┐  ┌────────┐  │
                │  │  bindy  │  │ bindcar  │  │  byob9 │  │
                │  │operator │  │  drone   │  │ (new)  │  │
                │  └────┬────┘  └────┬─────┘  └───┬────┘  │
                └───────┼────────────┼─────────────┼───────┘
                        │  rndc/nsup │             │ reads
                        ▼            ▼             ▼
                  ┌──────────────────────────────────────┐
                  │         Remote BIND9 instance         │
                  │  named.conf  zone files  rndc port   │
                  └──────────────────────────────────────┘
```

| Tool | Direction | Purpose |
|------|-----------|---------|
| **bindcar drone** | cluster → BIND9 | Manage a remote BIND9 via RNDC / nsupdate |
| **scout** (bindy) | k8s Ingress/Service → cluster | Create ARecords from annotated workloads |
| **byob9** | BIND9 named.conf → cluster | **One-time (Phase 1) or live-sync (Phase 2) import** |

---

## Binary Name Candidates

Following the bee-colony naming theme of the ecosystem:

| Candidate | Rationale |
|-----------|-----------|
| `forage` | Forager bees collect from external sources and bring back to the hive |
| `recruit` | Recruits an external BIND9 into the fleet |
| `waggle` | Waggle-dance communicates the location of an external resource |

**Recommended:** `forage` — descriptive, thematically consistent, one word.

The binary may live as a new crate in the bindcar workspace or as a standalone
repository linked from bindcar and bindy.

---

## Phase 1 — One-Time Import (MVP)

### Goal

Read `named.conf` → emit `DNSZone` + record CRD YAML to stdout. User pipes
to `kubectl apply -f -`. Idempotent (bindy operator handles conflicts).

### Inputs

| Flag / Env | Default | Description |
|------------|---------|-------------|
| `--conf <path>` | `/etc/bind/named.conf` | Path to `named.conf` |
| `--zone-dir <path>` | value of `directory` in `named.conf.options` | Base dir for resolving relative zone-file paths |
| `--namespace <ns>` | `bindy-system` | Kubernetes namespace for emitted resources |
| `--cluster-ref <name>` | *(none)* | Value to set on `DNSZoneSpec.cluster_ref` |
| `--zone-filter <glob>` | `*` | Only export zones matching this glob (e.g. `*.example.com`) |
| `--skip-records` | false | Emit only `DNSZone`, no record CRs |
| `--record-types <list>` | all | Comma-separated list: `A,AAAA,CNAME,MX,TXT,SRV,CAA,NS` |
| `--output <fmt>` | `yaml` | Output format: `yaml` or `json` |
| `--dry-run` | false | Print what would be emitted without reading zone files |

### Dependency on hornet

`hornet` (crates.io or path dep `~/dev/hornet`) provides:
- `parse_named_conf_file(path)` → `NamedConf` (zone names, types, file paths)
- `parse_zone_file_from_path(path)` → `ZoneFile` (all resource records)

### Data Mapping

#### `named.conf` Zone → `DNSZone` CR

| hornet field | bindy field | Notes |
|---|---|---|
| `ZoneStmt.name` | `spec.zoneName` | |
| `$TTL` from zone file | `spec.ttl` | Falls back to SOA minimum |
| `SoaData.mname` | `spec.soaRecord.primaryNs` | Ensure trailing dot |
| `SoaData.rname` | `spec.soaRecord.adminEmail` | Ensure trailing dot |
| `SoaData.serial` | `spec.soaRecord.serial` | |
| `SoaData.refresh` | `spec.soaRecord.refresh` | |
| `SoaData.retry` | `spec.soaRecord.retry` | |
| `SoaData.expire` | `spec.soaRecord.expire` | |
| `SoaData.minimum` | `spec.soaRecord.negativeTtl` | |
| `NS` records in zone | `spec.nameServers[].hostname` | Pair with A glue records |
| A records for NS names | `spec.nameServers[].ipv4Address` | Glue records |

`metadata.name` = zone name with dots replaced by hyphens, truncated to 253 chars
`spec.recordsFrom` = one entry with `matchLabels: { "bindy.firestoned.io/zone": "<zone-name>" }`

#### Zone file records → Record CRs

| hornet `RData` | bindy CR kind | Mapping notes |
|---|---|---|
| `A(Ipv4Addr)` | `ARecord` | `spec.name`, `spec.ipv4Addresses: [addr]` |
| `Aaaa(Ipv6Addr)` | `AAAARecord` | `spec.name`, `spec.ipv6Addresses: [addr]` |
| `Cname(Name)` | `CNAMERecord` | `spec.name`, `spec.alias` |
| `Mx(MxData)` | `MXRecord` | `spec.name`, `spec.preference`, `spec.exchange` |
| `Txt(Vec<String>)` | `TXTRecord` | Join strings; `spec.name`, `spec.value` |
| `Srv(SrvData)` | `SRVRecord` | `spec.name`, `spec.priority`, `spec.weight`, `spec.port`, `spec.target` |
| `Caa(CaaData)` | `CAARecord` | `spec.name`, `spec.flags`, `spec.tag`, `spec.value` |
| `Ns(Name)` | Folded into `DNSZone.spec.nameServers` | Not a standalone `NSRecord` CR |
| `Soa(SoaData)` | Folded into `DNSZone.spec.soaRecord` | Not a standalone record CR |
| Other types | Skipped (warn to stderr) | PTR, DNSSEC, LOC, etc. |

**Common metadata for all record CRs:**
```yaml
metadata:
  name: forage-{zone-slug}-{record-name}-{type}-{index}
  namespace: {namespace}
  labels:
    bindy.firestoned.io/managed-by: forage
    bindy.firestoned.io/zone: "{zone-name}"
    bindy.firestoned.io/source: named-conf-import
```

**Record deduplication / grouping:**
Multiple `A` records for the same name (e.g. `www A 1.2.3.4` and `www A 5.6.7.8`)
should be collapsed into a single `ARecord` CR with `spec.ipv4Addresses: [1.2.3.4, 5.6.7.8]`.
Same for `AAAA`. All other types emit one CR per record.

### Output Format (Phase 1)

All resources are emitted as a YAML stream separated by `---`. Order:
1. `DNSZone` resources (one per zone)
2. Record CRs (all record types, sorted by zone then record name)

Example:
```yaml
---
apiVersion: bindy.firestoned.io/v1beta1
kind: DNSZone
metadata:
  name: example-com
  namespace: bindy-system
  labels:
    bindy.firestoned.io/managed-by: forage
    bindy.firestoned.io/source: named-conf-import
spec:
  zoneName: example.com
  ttl: 3600
  soaRecord:
    primaryNs: "ns1.example.com."
    adminEmail: "admin.example.com."
    serial: 2024031501
    refresh: 3600
    retry: 600
    expire: 604800
    negativeTtl: 86400
  nameServers:
    - hostname: "ns1.example.com."
      ipv4Address: "192.0.2.1"
    - hostname: "ns2.example.com."
      ipv4Address: "192.0.2.253"
  recordsFrom:
    - selector:
        matchLabels:
          bindy.firestoned.io/zone: example.com
---
apiVersion: bindy.firestoned.io/v1beta1
kind: ARecord
metadata:
  name: forage-example-com-www-a-0
  namespace: bindy-system
  labels:
    bindy.firestoned.io/managed-by: forage
    bindy.firestoned.io/zone: example.com
    bindy.firestoned.io/source: named-conf-import
spec:
  name: www
  ipv4Addresses:
    - 203.0.113.10
  ttl: 3600
```

### Error Handling

- Zone referenced in `named.conf` but zone file not found → warn to stderr, skip records, still emit `DNSZone` shell
- Zone file parse error → warn to stderr with file path and line, skip that zone's records
- Unknown record type → debug log, skip silently
- `include` directives in `named.conf` → hornet resolves them; if include file missing → warn and continue

---

## Phase 2 — Live Sync (Future)

### Goal

Run as a long-lived process. Watch the `named.conf` and zone files for
changes (inotify / polling fallback). When a change is detected, reconcile
the delta against the cluster — creating, updating, or deleting bindy CRs
as needed.

### Design Sketch

```
┌──────────────────────────────────────────────────────────────┐
│  forage --sync --conf /etc/bind/named.conf                    │
│                                                               │
│  ┌─────────────┐    ┌───────────────┐    ┌────────────────┐  │
│  │  file watch │───▶│ hornet parser │───▶│ reconciler     │  │
│  │ (inotify /  │    │ (named.conf + │    │ diff current   │  │
│  │  poll)      │    │  zone files)  │    │ vs cluster CRs │  │
│  └─────────────┘    └───────────────┘    └───────┬────────┘  │
│                                                  │            │
│                                    ┌─────────────▼──────┐    │
│                                    │  kube client        │    │
│                                    │  apply / delete CRs │    │
│                                    └────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### Key Phase 2 Requirements

- **Idempotent reconciler**: Compare desired state (from hornet parse) to
  observed state (from kube list with `forage` label selector). Only issue
  patch/create/delete calls when there is a diff.
- **Debounce**: File-change events are noisy (editors write multiple times).
  Debounce by 500ms before re-parsing.
- **Kubernetes client**: Use `kube-rs` with in-cluster config by default;
  `--kubeconfig` flag for out-of-cluster use.
- **Leader election**: Not needed for Phase 2 since it runs as a single
  sidecar process alongside the BIND9 instance.
- **Metrics**: Expose Prometheus metrics at `/metrics` — zones watched, CRs
  created/updated/deleted, parse errors, reconcile duration.
- **Finalizer management**: Add a finalizer to CRs so they are cleaned up
  when `forage --sync` terminates gracefully.

---

## Implementation Plan

### Phase 1 Milestones

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | Create new crate `forage` (binary) in workspace or standalone | ✅ | Standalone repository (ADR-0001) |
| 2 | Implement `named.conf` → `Vec<DNSZone>` mapper | ✅ | `src/mapper.rs` |
| 3 | Implement zone-file record → record CR mapper | ✅ | A, AAAA, CNAME, MX, TXT, SRV, CAA. CNAME/MX/TXT field names were wrong against bindy until 2026-10-05 (ADR-0002) |
| 4 | YAML serializer for bindy CRD types | ✅ | Hand-written structs (ADR-0002) rendered by forage's own JSON/YAML writers, no serde (ADR-0006) |
| 5 | CLI with flags from Inputs table above | 🔶 | Own argument parser, no clap (ADR-0006). All flags except `--dry-run` → roadmap 01 |
| 6 | Unit tests: one zone file per record type | ✅ | `tests/fixtures/basic` covers every type in one zone; `tests/cli_tests.rs`, `src/crd_tests.rs` |
| 7 | Integration test against a real API server | ✅ | Superseded the "real-world sample" idea: e2e against pinned bindy CRDs (ADR-0003) |
| 8 | Add `forage` to bindcar CI (`make forage-build`) | 📄 | Superseded: forage has its own Build workflow (ADR-0004) |

### Phase 2 Milestones (future)

| # | Task |
|---|------|
| 1 | Add file-watcher (tokio + inotify via `notify` crate) |
| 2 | Implement reconciler: diff desired vs observed CRs |
| 3 | Add kube client + apply/patch/delete logic |
| 4 | Add `--sync` mode flag and long-running event loop |
| 5 | Add Prometheus metrics |
| 6 | Kubernetes Deployment manifest for sidecar mode |

---

## Open Questions

1. **Where does `forage` live?** *Resolved: standalone repository (ADR-0001).*
   - Option A: New binary in the `bindcar` workspace (`src/bin/forage.rs` or sub-crate)
   - Option B: New standalone repository `forage` (keeps it independent)
   - Option C: New binary in the `bindy` repository (closer to the CRD types)
   - *Recommendation*: Standalone repo — it has its own dependencies (hornet) and release cycle, but links conceptually to both bindcar (out-of-cluster) and bindy (CRD target).

2. **How to serialize bindy CRD types without importing the full `bindy` crate?** *Resolved: option A, hand-written structs (ADR-0002), not C.*
   - Option A: Copy the relevant `Spec` structs + derive `Serialize` (duplicates types)
   - Option B: Depend on a `bindy-types` crate extracted from bindy (needs bindy refactor)
   - Option C: Build raw `serde_json::Value` / `serde_yaml::Value` and emit without strong types
   - *Recommendation*: Option C for Phase 1 (fastest, no cross-repo coupling). Option B for Phase 2.

3. **Zone file path resolution** — `named.conf` `file` option can be relative to the `directory` option or absolute. The tool should resolve in this order: absolute path → relative to `--zone-dir` → relative to `named.conf` parent directory.

4. **`$INCLUDE` directives in zone files** — hornet handles these. If files are on a remote host (only `named.conf` is available locally), records will be incomplete. Document this limitation.

5. **Handling zones without zone files** (secondary, stub, forward) — these have no local zone files. Emit `DNSZone` with SOA zeroed out and a `forage.io/zone-type: secondary` annotation so the operator knows not to manage the SOA.

6. **`view` blocks in `named.conf`** — hornet parses `ViewStmt` which contains its own `ZoneStmt` list. Should zones inside views be imported? Proposed: skip views in Phase 1 (log a warning), support in Phase 2.

---

## Related Work

| Project | Relationship |
|---------|-------------|
| `hornet` (`~/dev/hornet`) | Parser library forage used until ADR-0006 replaced it with its own reader |
| `bindy` (`~/dev/bindy`) | CRD target — `forage` emits resources for the bindy operator |
| `bindcar drone` | Management plane — once onboarded, bindcar drone manages the same remote BIND9 |
| `scout` (bindy) | Conceptual sibling — scout creates CRs from k8s workloads; forage creates CRs from BIND9 |
