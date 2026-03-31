// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Maps hornet AST types to bindy CRD manifests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hornet_bind9::ast::named_conf::{NamedConf, Statement, ZoneStmt, ZoneType};
use hornet_bind9::ast::zone_file::{Entry, RData, ResourceRecord};
use serde_json::Value;
use tracing::{debug, warn};

use crate::crd::{
    AaaaRecord, AaaaRecordSpec, ARecord, ARecordSpec, CaaRecord, CaaRecordSpec, CnameRecord,
    CnameRecordSpec, DnsZone, DnsZoneSpec, LabelSelector, MxRecord, MxRecordSpec, NameServer,
    ObjectMeta, RecordSource, SoaRecord, SrvRecord, SrvRecordSpec, TxtRecord, TxtRecordSpec,
    API_VERSION, LABEL_MANAGED_BY, LABEL_SOURCE, LABEL_ZONE, MANAGED_BY_VALUE, SOURCE_VALUE,
};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for the [`Mapper`].
pub struct MapperConfig {
    /// Path to the named.conf file (used to resolve relative zone-file paths).
    pub conf_path: PathBuf,
    /// Explicit zone-file base directory override.
    pub zone_dir: Option<PathBuf>,
    /// Kubernetes namespace for emitted resources.
    pub namespace: String,
    /// Optional value for `DNSZoneSpec.cluster_ref`.
    pub cluster_ref: Option<String>,
    /// Glob-style zone name filter (e.g. `"*.example.com"`). `"*"` = all.
    pub zone_filter: String,
    /// When true, only `DNSZone` resources are emitted.
    pub skip_records: bool,
    /// If `Some`, only record types in this list are emitted.
    pub record_types: Option<Vec<String>>,
}

// ── Mapper ────────────────────────────────────────────────────────────────────

/// Converts a parsed [`NamedConf`] into a list of serialisable Kubernetes manifest values.
pub struct Mapper {
    config: MapperConfig,
}

impl Mapper {
    /// Create a new mapper with the given configuration.
    pub fn new(config: MapperConfig) -> Self {
        Self { config }
    }

    /// Map a [`NamedConf`] AST to a list of JSON values representing Kubernetes manifests.
    ///
    /// The returned values are ordered: all `DNSZone` resources first, then record CRs.
    pub fn map(&self, conf: &NamedConf) -> Result<Vec<Value>> {
        // Determine base directory for zone-file path resolution.
        let base_dir = self.resolve_base_dir(conf);
        debug!("zone file base directory: {}", base_dir.display());

        let mut zone_manifests: Vec<Value> = Vec::new();
        let mut record_manifests: Vec<Value> = Vec::new();

        for stmt in &conf.statements {
            let Statement::Zone(zone) = stmt else {
                continue;
            };

            if !self.zone_matches(&zone.name) {
                debug!("skipping zone '{}' (filtered out)", zone.name);
                continue;
            }

            if is_non_local_zone(zone) {
                warn!(
                    "zone '{}' is type {} — no local zone file; emitting DNSZone shell only",
                    zone.name,
                    zone.options
                        .zone_type
                        .as_ref()
                        .map(|t| format!("{t}"))
                        .unwrap_or_else(|| "unknown".into())
                );
                let dns_zone = self.build_dns_zone_shell(zone);
                zone_manifests.push(serde_json::to_value(dns_zone)?);
                continue;
            }

            // Resolve the zone file path.
            let zone_file_path = match self.resolve_zone_file(zone, &base_dir) {
                Some(p) => p,
                None => {
                    warn!(
                        "zone '{}' has no 'file' option — skipping records",
                        zone.name
                    );
                    let dns_zone = self.build_dns_zone_shell(zone);
                    zone_manifests.push(serde_json::to_value(dns_zone)?);
                    continue;
                }
            };

            // Parse the zone file.
            let zone_file = match hornet_bind9::parse_zone_file_from_path(&zone_file_path) {
                Ok(zf) => zf,
                Err(e) => {
                    warn!(
                        "failed to parse zone file '{}' for zone '{}': {e}",
                        zone_file_path.display(),
                        zone.name
                    );
                    let dns_zone = self.build_dns_zone_shell(zone);
                    zone_manifests.push(serde_json::to_value(dns_zone)?);
                    continue;
                }
            };

            // Extract the SOA record (required for DNSZone).
            let soa = extract_soa(&zone_file);

            // Collect NS records and their A glue records.
            let name_servers = extract_name_servers(&zone_file, &zone.name);

            // Build the DNSZone manifest.
            let dns_zone = self.build_dns_zone(zone, soa, name_servers, extract_ttl(&zone_file));
            zone_manifests.push(serde_json::to_value(dns_zone)?);

            if self.config.skip_records {
                continue;
            }

            // Build record CRs from the zone file.
            let records = self.build_record_manifests(&zone.name, &zone_file)?;
            record_manifests.extend(records);
        }

        let mut all = zone_manifests;
        all.extend(record_manifests);
        Ok(all)
    }

    // ── Zone file path resolution ─────────────────────────────────────────────

    fn resolve_base_dir(&self, conf: &NamedConf) -> PathBuf {
        // Priority: explicit --zone-dir flag → `directory` option in named.conf → parent dir of named.conf
        if let Some(ref d) = self.config.zone_dir {
            return d.clone();
        }
        for stmt in &conf.statements {
            if let Statement::Options(opts) = stmt {
                if let Some(ref dir) = opts.directory {
                    return PathBuf::from(dir);
                }
            }
        }
        self.config
            .conf_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    }

    fn resolve_zone_file(&self, zone: &ZoneStmt, base_dir: &Path) -> Option<PathBuf> {
        let file = zone.options.file.as_deref()?;
        let path = Path::new(file);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            Some(base_dir.join(path))
        }
    }

    // ── Zone filter ───────────────────────────────────────────────────────────

    fn zone_matches(&self, zone_name: &str) -> bool {
        let pattern = &self.config.zone_filter;
        if pattern == "*" {
            return true;
        }
        // Simple glob: only supports leading/trailing `*`
        if let Some(suffix) = pattern.strip_prefix('*') {
            return zone_name.ends_with(suffix);
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return zone_name.starts_with(prefix);
        }
        pattern == zone_name
    }

    fn record_type_allowed(&self, rtype: &str) -> bool {
        match &self.config.record_types {
            None => true,
            Some(types) => types.iter().any(|t| t == rtype),
        }
    }

    // ── DNSZone builders ──────────────────────────────────────────────────────

    fn build_dns_zone(
        &self,
        zone: &ZoneStmt,
        soa: Option<SoaRecord>,
        name_servers: Vec<NameServer>,
        ttl: Option<i32>,
    ) -> DnsZone {
        let soa_record = soa.unwrap_or_else(|| SoaRecord {
            primary_ns: format!("ns1.{}.", zone.name),
            admin_email: format!("admin.{}.", zone.name),
            serial: 0,
            refresh: 3600,
            retry: 600,
            expire: 604800,
            negative_ttl: 86400,
        });

        DnsZone {
            api_version: API_VERSION.to_string(),
            kind: "DNSZone".to_string(),
            metadata: self.zone_meta(&zone.name),
            spec: DnsZoneSpec {
                zone_name: zone.name.clone(),
                soa_record,
                ttl,
                cluster_ref: self.config.cluster_ref.clone(),
                name_servers,
                records_from: vec![RecordSource {
                    selector: LabelSelector {
                        match_labels: BTreeMap::from([(
                            LABEL_ZONE.to_string(),
                            zone.name.clone(),
                        )]),
                    },
                }],
            },
        }
    }

    fn build_dns_zone_shell(&self, zone: &ZoneStmt) -> DnsZone {
        self.build_dns_zone(zone, None, vec![], None)
    }

    fn zone_meta(&self, zone_name: &str) -> ObjectMeta {
        ObjectMeta {
            name: zone_slug(zone_name),
            namespace: self.config.namespace.clone(),
            labels: BTreeMap::from([
                (LABEL_MANAGED_BY.to_string(), MANAGED_BY_VALUE.to_string()),
                (LABEL_SOURCE.to_string(), SOURCE_VALUE.to_string()),
            ]),
            annotations: BTreeMap::new(),
        }
    }

    fn record_meta(&self, name: &str, zone_name: &str) -> ObjectMeta {
        ObjectMeta {
            name: name.to_string(),
            namespace: self.config.namespace.clone(),
            labels: BTreeMap::from([
                (LABEL_MANAGED_BY.to_string(), MANAGED_BY_VALUE.to_string()),
                (LABEL_ZONE.to_string(), zone_name.to_string()),
                (LABEL_SOURCE.to_string(), SOURCE_VALUE.to_string()),
            ]),
            annotations: BTreeMap::new(),
        }
    }

    // ── Record CR builders ────────────────────────────────────────────────────

    fn build_record_manifests(
        &self,
        zone_name: &str,
        zone_file: &hornet_bind9::ast::zone_file::ZoneFile,
    ) -> Result<Vec<Value>> {
        // Group A and AAAA records by name to collapse multi-address records.
        let mut a_records: BTreeMap<(String, Option<i32>), Vec<String>> = BTreeMap::new();
        let mut aaaa_records: BTreeMap<(String, Option<i32>), Vec<String>> = BTreeMap::new();
        let mut other_records: Vec<Value> = Vec::new();
        let mut index_counters: BTreeMap<String, usize> = BTreeMap::new();

        for rr in zone_file.records() {
            let record_name = resolve_record_name(rr, zone_name);
            let ttl = rr.ttl.map(|t| t as i32);

            match &rr.rdata {
                RData::A(addr) if self.record_type_allowed("A") => {
                    a_records
                        .entry((record_name, ttl))
                        .or_default()
                        .push(addr.to_string());
                }
                RData::Aaaa(addr) if self.record_type_allowed("AAAA") => {
                    aaaa_records
                        .entry((record_name, ttl))
                        .or_default()
                        .push(addr.to_string());
                }
                RData::Cname(target) if self.record_type_allowed("CNAME") => {
                    let idx = next_index(&mut index_counters, "cname");
                    let cr_name =
                        cr_name("forage", zone_name, &record_name, "cname", idx);
                    let cr = CnameRecord {
                        api_version: API_VERSION.to_string(),
                        kind: "CNAMERecord".to_string(),
                        metadata: self.record_meta(&cr_name, zone_name),
                        spec: CnameRecordSpec {
                            name: record_name,
                            alias: ensure_trailing_dot(target.as_str()),
                            ttl,
                        },
                    };
                    other_records.push(serde_json::to_value(cr)?);
                }
                RData::Mx(mx) if self.record_type_allowed("MX") => {
                    let idx = next_index(&mut index_counters, "mx");
                    let cr_name = cr_name("forage", zone_name, &record_name, "mx", idx);
                    let cr = MxRecord {
                        api_version: API_VERSION.to_string(),
                        kind: "MXRecord".to_string(),
                        metadata: self.record_meta(&cr_name, zone_name),
                        spec: MxRecordSpec {
                            name: record_name,
                            preference: mx.preference,
                            exchange: ensure_trailing_dot(mx.exchange.as_str()),
                            ttl,
                        },
                    };
                    other_records.push(serde_json::to_value(cr)?);
                }
                RData::Txt(parts) if self.record_type_allowed("TXT") => {
                    let idx = next_index(&mut index_counters, "txt");
                    let cr_name = cr_name("forage", zone_name, &record_name, "txt", idx);
                    let cr = TxtRecord {
                        api_version: API_VERSION.to_string(),
                        kind: "TXTRecord".to_string(),
                        metadata: self.record_meta(&cr_name, zone_name),
                        spec: TxtRecordSpec {
                            name: record_name,
                            value: parts.join(""),
                            ttl,
                        },
                    };
                    other_records.push(serde_json::to_value(cr)?);
                }
                RData::Srv(srv) if self.record_type_allowed("SRV") => {
                    let idx = next_index(&mut index_counters, "srv");
                    let cr_name = cr_name("forage", zone_name, &record_name, "srv", idx);
                    let cr = SrvRecord {
                        api_version: API_VERSION.to_string(),
                        kind: "SRVRecord".to_string(),
                        metadata: self.record_meta(&cr_name, zone_name),
                        spec: SrvRecordSpec {
                            name: record_name,
                            priority: srv.priority,
                            weight: srv.weight,
                            port: srv.port,
                            target: ensure_trailing_dot(srv.target.as_str()),
                            ttl,
                        },
                    };
                    other_records.push(serde_json::to_value(cr)?);
                }
                RData::Caa(caa) if self.record_type_allowed("CAA") => {
                    let idx = next_index(&mut index_counters, "caa");
                    let cr_name = cr_name("forage", zone_name, &record_name, "caa", idx);
                    let cr = CaaRecord {
                        api_version: API_VERSION.to_string(),
                        kind: "CAARecord".to_string(),
                        metadata: self.record_meta(&cr_name, zone_name),
                        spec: CaaRecordSpec {
                            name: record_name,
                            flags: caa.flags,
                            tag: caa.tag.clone(),
                            value: caa.value.clone(),
                            ttl,
                        },
                    };
                    other_records.push(serde_json::to_value(cr)?);
                }
                RData::Ns(_) | RData::Soa(_) => {
                    // Folded into DNSZone — not standalone record CRs.
                }
                other => {
                    debug!(
                        "skipping unsupported record type '{}' for name '{}'",
                        other.rtype(),
                        record_name
                    );
                }
            }
        }

        // Emit collapsed A records.
        let mut result: Vec<Value> = Vec::new();
        for ((name, ttl), addrs) in a_records {
            let idx = next_index(&mut index_counters, "a");
            let cr_name = cr_name("forage", zone_name, &name, "a", idx);
            let cr = ARecord {
                api_version: API_VERSION.to_string(),
                kind: "ARecord".to_string(),
                metadata: self.record_meta(&cr_name, zone_name),
                spec: ARecordSpec {
                    name,
                    ipv4_addresses: addrs,
                    ttl,
                },
            };
            result.push(serde_json::to_value(cr)?);
        }

        // Emit collapsed AAAA records.
        for ((name, ttl), addrs) in aaaa_records {
            let idx = next_index(&mut index_counters, "aaaa");
            let cr_name = cr_name("forage", zone_name, &name, "aaaa", idx);
            let cr = AaaaRecord {
                api_version: API_VERSION.to_string(),
                kind: "AAAARecord".to_string(),
                metadata: self.record_meta(&cr_name, zone_name),
                spec: AaaaRecordSpec {
                    name,
                    ipv6_addresses: addrs,
                    ttl,
                },
            };
            result.push(serde_json::to_value(cr)?);
        }

        result.extend(other_records);
        Ok(result)
    }
}

// ── Zone file helpers ─────────────────────────────────────────────────────────

/// Returns true for zone types that have no local zone file.
fn is_non_local_zone(zone: &ZoneStmt) -> bool {
    matches!(
        zone.options.zone_type,
        Some(ZoneType::Secondary)
            | Some(ZoneType::Stub)
            | Some(ZoneType::Forward)
            | Some(ZoneType::Hint)
    )
}

/// Extract the SOA record from a zone file, if present.
fn extract_soa(
    zone_file: &hornet_bind9::ast::zone_file::ZoneFile,
) -> Option<SoaRecord> {
    zone_file.records().find_map(|rr| {
        if let RData::Soa(soa) = &rr.rdata {
            Some(SoaRecord {
                primary_ns: ensure_trailing_dot(soa.mname.as_str()),
                admin_email: ensure_trailing_dot(soa.rname.as_str()),
                serial: soa.serial as i64,
                refresh: soa.refresh as i32,
                retry: soa.retry as i32,
                expire: soa.expire as i32,
                negative_ttl: soa.minimum as i32,
            })
        } else {
            None
        }
    })
}

/// Extract the $TTL value from zone file directives.
fn extract_ttl(zone_file: &hornet_bind9::ast::zone_file::ZoneFile) -> Option<i32> {
    zone_file.entries.iter().find_map(|e| {
        if let Entry::Ttl(ttl) = e {
            Some(*ttl as i32)
        } else {
            None
        }
    })
}

/// Extract NS records and pair them with A glue records to build [`NameServer`] entries.
fn extract_name_servers(
    zone_file: &hornet_bind9::ast::zone_file::ZoneFile,
    zone_name: &str,
) -> Vec<NameServer> {
    // Collect NS hostnames.
    let ns_names: Vec<String> = zone_file
        .records()
        .filter_map(|rr| {
            if let RData::Ns(name) = &rr.rdata {
                Some(ensure_trailing_dot(name.as_str()))
            } else {
                None
            }
        })
        .collect();

    // Collect A glue records: name → IPv4.
    let glue_a: BTreeMap<String, String> = zone_file
        .records()
        .filter_map(|rr| {
            if let RData::A(addr) = &rr.rdata {
                let name = resolve_record_name(rr, zone_name);
                Some((format!("{name}.{zone_name}."), addr.to_string()))
            } else {
                None
            }
        })
        .collect();

    ns_names
        .into_iter()
        .map(|hostname| {
            let ipv4_address = glue_a.get(&hostname).cloned();
            NameServer {
                hostname,
                ipv4_address,
                ipv6_address: None,
            }
        })
        .collect()
}

// ── Name utilities ────────────────────────────────────────────────────────────

/// Resolve a resource record's name relative to the zone name.
/// `@` and absent names → `@` (apex). Absolute names have the zone suffix stripped.
fn resolve_record_name(rr: &ResourceRecord, zone_name: &str) -> String {
    use hornet_bind9::ast::zone_file::Name;
    match &rr.name {
        None => "@".to_string(),
        Some(name) if Name::is_at(name) => "@".to_string(),
        Some(name) => {
            let s = name.as_str();
            // Strip trailing zone suffix if the name is absolute.
            let zone_suffix = format!(".{zone_name}.");
            if s.ends_with(&zone_suffix) {
                s[..s.len() - zone_suffix.len()].to_string()
            } else if s == format!("{zone_name}.") {
                "@".to_string()
            } else {
                s.trim_end_matches('.').to_string()
            }
        }
    }
}

/// Convert a zone name like `example.com` into a Kubernetes-safe slug `example-com`.
fn zone_slug(zone_name: &str) -> String {
    sanitize_k8s_name(zone_name)
}

/// Generate a deterministic CR name.
fn cr_name(prefix: &str, zone: &str, record: &str, rtype: &str, index: usize) -> String {
    let base = format!("{prefix}-{zone}-{record}-{rtype}-{index}");
    sanitize_k8s_name(&base)
}

/// Sanitize a string to be a valid Kubernetes name: lowercase, hyphens for non-alphanumerics, max 253 chars.
fn sanitize_k8s_name(s: &str) -> String {
    const MAX_K8S_NAME_LEN: usize = 253;
    let result: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    // Trim leading/trailing hyphens and collapse runs.
    let result = result.trim_matches('-').to_string();
    if result.len() > MAX_K8S_NAME_LEN {
        result[..MAX_K8S_NAME_LEN].to_string()
    } else {
        result
    }
}

/// Ensure a DNS name ends with a trailing dot (required by bindy).
fn ensure_trailing_dot(s: &str) -> String {
    if s.ends_with('.') {
        s.to_string()
    } else {
        format!("{s}.")
    }
}

fn next_index(counters: &mut BTreeMap<String, usize>, key: &str) -> usize {
    let entry = counters.entry(key.to_string()).or_insert(0);
    let idx = *entry;
    *entry += 1;
    idx
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "mapper_test.rs"]
mod mapper_test;
