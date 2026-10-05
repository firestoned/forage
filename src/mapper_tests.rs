// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use crate::named_conf::parse_named_conf;
use crate::test_support::TempDir;

use super::*;
use crate::crd::{LABEL_MANAGED_BY, LABEL_ZONE, MANAGED_BY_VALUE};

fn default_config() -> MapperConfig {
    MapperConfig {
        conf_path: PathBuf::from("/etc/bind/named.conf"),
        zone_dir: None,
        namespace: "bindy-system".to_string(),
        cluster_ref: None,
        zone_filter: "*".to_string(),
        skip_records: false,
        record_types: None,
    }
}

// ── zone_slug ────────────────────────────────────────────────────────────────

#[test]
fn test_zone_slug_replaces_dots_with_hyphens() {
    assert_eq!(zone_slug("example.com"), "example-com");
}

#[test]
fn test_zone_slug_handles_subdomains() {
    assert_eq!(zone_slug("sub.example.com"), "sub-example-com");
}

// ── sanitize_k8s_name ────────────────────────────────────────────────────────

#[test]
fn test_sanitize_k8s_name_lowercases() {
    assert_eq!(sanitize_k8s_name("EXAMPLE"), "example");
}

#[test]
fn test_sanitize_k8s_name_replaces_special_chars() {
    assert_eq!(sanitize_k8s_name("foo_bar.baz"), "foo-bar-baz");
}

#[test]
fn test_sanitize_k8s_name_trims_hyphens() {
    assert_eq!(sanitize_k8s_name(".example."), "example");
}

// ── ensure_trailing_dot ──────────────────────────────────────────────────────

#[test]
fn test_ensure_trailing_dot_adds_dot() {
    assert_eq!(ensure_trailing_dot("ns1.example.com"), "ns1.example.com.");
}

#[test]
fn test_ensure_trailing_dot_idempotent() {
    assert_eq!(ensure_trailing_dot("ns1.example.com."), "ns1.example.com.");
}

// ── resolve_record_name ──────────────────────────────────────────────────────

#[test]
fn test_resolve_record_name_at() {
    let rr = ResourceRecord {
        name: Some(Name::new("@")),
        ttl: None,
        class: None,
        rdata: RData::A("1.2.3.4".parse().unwrap()),
    };
    assert_eq!(resolve_record_name(&rr, "example.com"), "@");
}

#[test]
fn test_resolve_record_name_relative() {
    let rr = ResourceRecord {
        name: Some(Name::new("www")),
        ttl: None,
        class: None,
        rdata: RData::A("1.2.3.4".parse().unwrap()),
    };
    assert_eq!(resolve_record_name(&rr, "example.com"), "www");
}

// ── zone_matches ─────────────────────────────────────────────────────────────

#[test]
fn test_zone_matches_wildcard_matches_all() {
    let mapper = Mapper::new(default_config());
    assert!(mapper.zone_matches("example.com"));
    assert!(mapper.zone_matches("other.org"));
}

#[test]
fn test_zone_matches_suffix_glob() {
    let mapper = Mapper::new(MapperConfig {
        zone_filter: "*.example.com".to_string(),
        ..default_config()
    });
    assert!(mapper.zone_matches("sub.example.com"));
    assert!(!mapper.zone_matches("other.org"));
}

#[test]
fn test_zone_matches_exact() {
    let mapper = Mapper::new(MapperConfig {
        zone_filter: "example.com".to_string(),
        ..default_config()
    });
    assert!(mapper.zone_matches("example.com"));
    assert!(!mapper.zone_matches("other.com"));
}

// ── map — no zone file (secondary zone) ─────────────────────────────────────

#[test]
fn test_map_secondary_zone_emits_dns_zone_shell() {
    let input = r#"
zone "example.com" {
    type secondary;
    primaries { 192.0.2.1; };
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(default_config());
    let manifests = mapper.map(&conf);

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0]["kind"], "DNSZone");
    assert_eq!(manifests[0]["spec"]["zoneName"], "example.com");
}

// ── map — primary zone with inline records via zone file ─────────────────────

#[test]
fn test_map_primary_zone_no_file_option() {
    // Zone with no `file` option → emits DNSZone shell, no records
    let input = r#"
zone "example.com" {
    type primary;
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(default_config());
    let manifests = mapper.map(&conf);

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0]["kind"], "DNSZone");
}

// ── map — skip_records flag ──────────────────────────────────────────────────

#[test]
fn test_map_skip_records_emits_only_dns_zones() {
    let input = r#"
zone "example.com" {
    type secondary;
    primaries { 192.0.2.1; };
};
zone "other.org" {
    type secondary;
    primaries { 192.0.2.2; };
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(MapperConfig {
        skip_records: true,
        ..default_config()
    });
    let manifests = mapper.map(&conf);

    assert_eq!(manifests.len(), 2);
    assert!(manifests.iter().all(|m| m["kind"] == "DNSZone"));
}

// ── map — zone filter ────────────────────────────────────────────────────────

#[test]
fn test_map_zone_filter_excludes_non_matching() {
    let input = r#"
zone "example.com" {
    type secondary;
    primaries { 192.0.2.1; };
};
zone "other.org" {
    type secondary;
    primaries { 192.0.2.2; };
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(MapperConfig {
        zone_filter: "*.com".to_string(),
        ..default_config()
    });
    let manifests = mapper.map(&conf);

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0]["spec"]["zoneName"], "example.com");
}

// ── DNSZone metadata ─────────────────────────────────────────────────────────

#[test]
fn test_dns_zone_has_correct_labels() {
    let input = r#"
zone "example.com" {
    type secondary;
    primaries { 192.0.2.1; };
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(default_config());
    let manifests = mapper.map(&conf);

    let labels = &manifests[0]["metadata"]["labels"];
    assert_eq!(labels[LABEL_MANAGED_BY], MANAGED_BY_VALUE);
}

// ── DNSZone recordsFrom ──────────────────────────────────────────────────────

#[test]
fn test_dns_zone_records_from_has_zone_label_selector() {
    let input = r#"
zone "example.com" {
    type secondary;
    primaries { 192.0.2.1; };
};
"#;
    let conf = parse_named_conf(input).expect("parse failed");
    let mapper = Mapper::new(default_config());
    let manifests = mapper.map(&conf);

    let selector = &manifests[0]["spec"]["recordsFrom"][0]["selector"]["matchLabels"];
    assert_eq!(selector[LABEL_ZONE], "example.com");
}

// ── Helpers: real named.conf + zone files in a temp dir ─────────────────────

/// A temp directory holding a `named.conf` and its zone files.
struct ConfDir {
    dir: TempDir,
}

impl ConfDir {
    fn new() -> Self {
        Self {
            dir: TempDir::new(),
        }
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        self.dir.write(name, content)
    }

    /// Writes `named.conf` and maps it with `config` (its `conf_path` is set here).
    fn map(&self, named_conf: &str, config: MapperConfig) -> Vec<Value> {
        let conf_path = self.write("named.conf", named_conf);
        let conf = parse_named_conf(named_conf).expect("parse failed");
        Mapper::new(MapperConfig {
            conf_path,
            ..config
        })
        .map(&conf)
    }
}

// SOA in the parenthesized form, the common style in real zone files. The
// one-line form is covered in `zone_file_tests.rs`.
const SOA_EXAMPLE_COM: &str = "\
$TTL 3600
@ IN SOA ns1.example.com. hostmaster.example.com. ( 7 3600 600 604800 300 )
@ IN NS ns1.example.com.
ns1 IN A 192.0.2.1
";

fn primary_zone(file: &str) -> String {
    format!("zone \"example.com\" {{ type primary; file \"{file}\"; }};\n")
}

fn kinds(manifests: &[Value]) -> Vec<&str> {
    manifests
        .iter()
        .map(|m| m["kind"].as_str().expect("kind"))
        .collect()
}

fn find<'a>(manifests: &'a [Value], kind: &str, name: &str) -> &'a Value {
    manifests
        .iter()
        .find(|m| m["kind"] == kind && m["spec"]["name"] == name)
        .unwrap_or_else(|| panic!("no {kind} named {name}"))
}

// ── map: zone files ───────────────────────────────────────────────────────────

#[test]
fn test_map_primary_zone_reads_soa_ttl_and_glue() {
    let dir = ConfDir::new();
    dir.write("db.example.com", SOA_EXAMPLE_COM);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let zone = &manifests[0];
    assert_eq!(zone["kind"], "DNSZone");
    assert_eq!(zone["spec"]["ttl"], 3600);
    assert_eq!(zone["spec"]["soaRecord"]["serial"], 7);
    assert_eq!(zone["spec"]["soaRecord"]["primaryNs"], "ns1.example.com.");
    assert_eq!(
        zone["spec"]["soaRecord"]["adminEmail"],
        "hostmaster.example.com."
    );
    assert_eq!(zone["spec"]["soaRecord"]["negativeTtl"], 300);
    assert_eq!(
        zone["spec"]["nameServers"][0]["hostname"],
        "ns1.example.com."
    );
    assert_eq!(zone["spec"]["nameServers"][0]["ipv4Address"], "192.0.2.1");
}

#[test]
fn test_map_name_server_without_glue_has_no_address() {
    let dir = ConfDir::new();
    dir.write(
        "db.example.com",
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\n@ IN NS ns.example.net.\n",
    );

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let ns = &manifests[0]["spec"]["nameServers"][0];
    assert_eq!(ns["hostname"], "ns.example.net.");
    assert!(ns.get("ipv4Address").is_none());
}

#[test]
fn test_map_zone_file_without_soa_or_ttl_uses_placeholder_soa() {
    let dir = ConfDir::new();
    dir.write("db.example.com", "www IN A 192.0.2.10\n");

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let zone = &manifests[0];
    assert!(zone["spec"].get("ttl").is_none());
    assert_eq!(zone["spec"]["soaRecord"]["serial"], 0);
    assert_eq!(zone["spec"]["soaRecord"]["primaryNs"], "ns1.example.com.");
    assert_eq!(kinds(&manifests), vec!["DNSZone", "ARecord"]);
}

#[test]
fn test_map_missing_zone_file_emits_shell_and_no_records() {
    let dir = ConfDir::new();

    let manifests = dir.map(&primary_zone("db.missing"), default_config());

    assert_eq!(kinds(&manifests), vec!["DNSZone"]);
    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 0);
}

#[test]
fn test_map_absolute_zone_file_path_is_used_as_is() {
    let dir = ConfDir::new();
    let zone_file = dir.write("zones/db.example.com", SOA_EXAMPLE_COM);

    let manifests = dir.map(
        &primary_zone(zone_file.to_str().expect("utf-8 path")),
        default_config(),
    );

    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 7);
}

#[test]
fn test_map_zone_dir_overrides_conf_parent() {
    let dir = ConfDir::new();
    dir.write("elsewhere/db.example.com", SOA_EXAMPLE_COM);

    let manifests = dir.map(
        &primary_zone("db.example.com"),
        MapperConfig {
            zone_dir: Some(dir.path().join("elsewhere")),
            ..default_config()
        },
    );

    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 7);
}

#[test]
fn test_map_directory_option_sets_base_dir_and_options_are_skipped() {
    let dir = ConfDir::new();
    dir.write("var/db.example.com", SOA_EXAMPLE_COM);
    let named_conf = format!(
        "options {{ directory \"{}\"; }};\n{}",
        dir.path().join("var").display(),
        primary_zone("db.example.com")
    );

    let manifests = dir.map(&named_conf, default_config());

    assert_eq!(kinds(&manifests), vec!["DNSZone", "ARecord"]);
    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 7);
}

#[test]
fn test_map_skip_records_with_zone_file_keeps_soa_but_drops_records() {
    let dir = ConfDir::new();
    dir.write("db.example.com", SOA_EXAMPLE_COM);

    let manifests = dir.map(
        &primary_zone("db.example.com"),
        MapperConfig {
            skip_records: true,
            ..default_config()
        },
    );

    assert_eq!(kinds(&manifests), vec!["DNSZone"]);
    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 7);
}

#[test]
fn test_map_cluster_ref_and_namespace_are_applied() {
    let dir = ConfDir::new();
    dir.write("db.example.com", SOA_EXAMPLE_COM);

    let manifests = dir.map(
        &primary_zone("db.example.com"),
        MapperConfig {
            namespace: "dns".to_string(),
            cluster_ref: Some("prod-dns".to_string()),
            ..default_config()
        },
    );

    assert_eq!(manifests[0]["spec"]["clusterRef"], "prod-dns");
    assert!(manifests
        .iter()
        .all(|m| m["metadata"]["namespace"] == "dns"));
}

// ── map: record types ────────────────────────────────────────────────────────

const EVERY_TYPE: &str = "\
$TTL 3600
@ IN SOA ns1.example.com. hostmaster.example.com. ( 1 3600 600 604800 300 )
@ IN NS ns1.example.com.
ns1 IN A 192.0.2.1
www IN A 192.0.2.10
www IN A 192.0.2.11
www IN AAAA 2001:db8::10
www IN AAAA 2001:db8::11
docs IN CNAME www
@ IN MX 10 mail.example.com.
_sip._tcp IN SRV 10 60 5060 sip.example.com.
@ IN CAA 0 issue \"letsencrypt.org\"
@ IN TXT \"v=spf1\" \" -all\"
";

#[test]
fn test_map_emits_every_supported_record_type() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    assert_eq!(
        kinds(&manifests),
        vec![
            "DNSZone",
            "ARecord",
            "ARecord",
            "AAAARecord",
            "CNAMERecord",
            "MXRecord",
            "SRVRecord",
            "CAARecord",
            "TXTRecord"
        ]
    );
}

#[test]
fn test_map_collapses_aaaa_addresses_per_name() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let www = find(&manifests, "AAAARecord", "www");
    assert_eq!(
        www["spec"]["ipv6Addresses"],
        Value::from(vec!["2001:db8::10", "2001:db8::11"])
    );
}

/// A relative CNAME target is relative to the zone origin: `docs CNAME www`
/// means `www.example.com.`. forage appends a bare dot instead, producing the
/// root-level name `www.` (roadmap 01). MX exchange, SRV target and NS names
/// go through the same helper.
#[test]
#[ignore = "relative targets are made absolute at the root, not the zone (roadmap 01)"]
fn test_map_relative_cname_target_is_qualified_with_the_zone() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let docs = find(&manifests, "CNAMERecord", "docs");
    assert_eq!(docs["spec"]["target"], "www.example.com.");
}

#[test]
fn test_map_absolute_cname_target_is_kept() {
    let dir = ConfDir::new();
    dir.write(
        "db.example.com",
        &format!("{SOA_EXAMPLE_COM}docs IN CNAME www.example.org.\n"),
    );

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let docs = find(&manifests, "CNAMERecord", "docs");
    assert_eq!(docs["spec"]["target"], "www.example.org.");
}

#[test]
fn test_map_srv_and_caa_fields() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let srv = find(&manifests, "SRVRecord", "_sip._tcp");
    assert_eq!(srv["spec"]["priority"], 10);
    assert_eq!(srv["spec"]["weight"], 60);
    assert_eq!(srv["spec"]["port"], 5060);
    assert_eq!(srv["spec"]["target"], "sip.example.com.");
    let caa = find(&manifests, "CAARecord", "@");
    assert_eq!(caa["spec"]["flags"], 0);
    assert_eq!(caa["spec"]["tag"], "issue");
    assert_eq!(caa["spec"]["value"], "letsencrypt.org");
}

#[test]
fn test_map_txt_keeps_character_strings_separate() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let txt = find(&manifests, "TXTRecord", "@");
    assert_eq!(txt["spec"]["text"], Value::from(vec!["v=spf1", " -all"]));
}

#[test]
fn test_map_record_types_filter_limits_output() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(
        &primary_zone("db.example.com"),
        MapperConfig {
            record_types: Some(vec!["MX".to_string(), "TXT".to_string()]),
            ..default_config()
        },
    );

    assert_eq!(kinds(&manifests), vec!["DNSZone", "MXRecord", "TXTRecord"]);
}

#[test]
fn test_map_record_types_filter_still_reads_ns_glue() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let manifests = dir.map(
        &primary_zone("db.example.com"),
        MapperConfig {
            record_types: Some(vec!["CAA".to_string()]),
            ..default_config()
        },
    );

    assert_eq!(
        manifests[0]["spec"]["nameServers"][0]["ipv4Address"],
        "192.0.2.1"
    );
}

#[test]
fn test_map_unsupported_record_types_are_skipped() {
    let dir = ConfDir::new();
    dir.write(
        "db.example.com",
        &format!("{SOA_EXAMPLE_COM}10 IN PTR host.example.com.\n"),
    );

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    assert_eq!(kinds(&manifests), vec!["DNSZone", "ARecord"]);
}

#[test]
fn test_map_record_ttl_is_carried_and_splits_a_groups() {
    let dir = ConfDir::new();
    dir.write(
        "db.example.com",
        &format!("{SOA_EXAMPLE_COM}api 300 IN A 192.0.2.20\napi 600 IN A 192.0.2.21\n"),
    );

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    let ttls: Vec<&Value> = manifests
        .iter()
        .filter(|m| m["kind"] == "ARecord" && m["spec"]["name"] == "api")
        .map(|m| &m["spec"]["ttl"])
        .collect();
    assert_eq!(ttls, vec![&Value::from(300), &Value::from(600)]);
}

// ── map: record names ────────────────────────────────────────────────────────

#[test]
fn test_map_absolute_owner_names_are_made_relative() {
    let dir = ConfDir::new();
    dir.write(
        "db.example.com",
        &format!(
            "{SOA_EXAMPLE_COM}www.example.com. IN A 192.0.2.30\nexample.com. IN MX 5 mx.example.com.\nhost.example.net. IN A 192.0.2.31\n"
        ),
    );

    let manifests = dir.map(&primary_zone("db.example.com"), default_config());

    find(&manifests, "ARecord", "www");
    find(&manifests, "MXRecord", "@");
    find(&manifests, "ARecord", "host.example.net");
}

fn rr_named(name: Option<&str>) -> ResourceRecord {
    ResourceRecord {
        name: name.map(Name::new),
        ttl: None,
        class: None,
        rdata: RData::A("192.0.2.1".parse().expect("ipv4")),
    }
}

#[test]
fn test_resolve_record_name_absent_owner_is_apex() {
    // The parser yields `None` only when the first record of a file has a
    // blank owner; the mapper treats it as the apex.
    assert_eq!(resolve_record_name(&rr_named(None), "example.com"), "@");
}

#[test]
fn test_resolve_record_name_branches() {
    for (owner, expected) in [
        ("@", "@"),
        ("www", "www"),
        ("www.example.com.", "www"),
        ("a.b.example.com.", "a.b"),
        ("example.com.", "@"),
        ("host.example.net.", "host.example.net"),
    ] {
        assert_eq!(
            resolve_record_name(&rr_named(Some(owner)), "example.com"),
            expected,
            "owner {owner}"
        );
    }
}

#[test]
fn test_map_cr_names_are_deterministic_and_indexed() {
    let dir = ConfDir::new();
    dir.write("db.example.com", EVERY_TYPE);

    let first = dir.map(&primary_zone("db.example.com"), default_config());
    let second = dir.map(&primary_zone("db.example.com"), default_config());

    assert_eq!(first, second);
    let names: Vec<&str> = first
        .iter()
        .filter(|m| m["kind"] == "ARecord")
        .map(|m| m["metadata"]["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        vec!["forage-example-com-ns1-a-0", "forage-example-com-www-a-1"]
    );
}

// ── zone filter / names: remaining branches ─────────────────────────────────

#[test]
fn test_zone_matches_prefix_glob() {
    let mapper = Mapper::new(MapperConfig {
        zone_filter: "example.*".to_string(),
        ..default_config()
    });
    assert!(mapper.zone_matches("example.com"));
    assert!(!mapper.zone_matches("other.example"));
}

#[test]
fn test_record_type_allowed_with_and_without_filter() {
    let all = Mapper::new(default_config());
    let only_a = Mapper::new(MapperConfig {
        record_types: Some(vec!["A".to_string()]),
        ..default_config()
    });
    assert!(all.record_type_allowed("TXT"));
    assert!(only_a.record_type_allowed("A"));
    assert!(!only_a.record_type_allowed("AAAA"));
}

#[test]
fn test_sanitize_k8s_name_truncates_to_253_chars() {
    let long = "a".repeat(300);
    assert_eq!(sanitize_k8s_name(&long).len(), 253);
}

/// Truncating at 253 can leave a trailing `-` (invalid Kubernetes name), and
/// drops the `-<type>-<index>` suffix, so two long owners with a common prefix
/// collide (roadmap 01).
#[test]
#[ignore = "truncated CR names can end in '-' and can collide (roadmap 01)"]
fn test_truncated_cr_names_are_valid_and_unique() {
    let prefix = format!("{}.", "a".repeat(240));
    let first = cr_name("forage", "example.com", &format!("{prefix}one"), "a", 0);
    let second = cr_name("forage", "example.com", &format!("{prefix}two"), "a", 1);

    assert!(first.len() <= 253);
    assert!(!first.ends_with('-'), "{first}");
    assert_ne!(first, second);
}

#[test]
fn test_cr_name_shape() {
    assert_eq!(
        cr_name("forage", "example.com", "www", "a", 3),
        "forage-example-com-www-a-3"
    );
}

#[test]
fn test_non_local_zone_type_covers_each_type() {
    for (zone_type, expected) in [
        ("primary", None),
        ("secondary", Some("secondary")),
        ("stub", Some("stub")),
        ("forward", Some("forward")),
        ("hint", Some("hint")),
    ] {
        let input = format!("zone \"example.com\" {{ type {zone_type}; }};");
        let conf = parse_named_conf(&input).expect("parse failed");
        let Statement::Zone(zone) = &conf.statements[0] else {
            panic!("not a zone statement");
        };
        assert_eq!(non_local_zone_type(zone), expected, "type {zone_type}");
    }
}

#[test]
fn test_non_local_zone_type_without_type_is_local() {
    let conf = parse_named_conf("zone \"example.com\" { file \"db\"; };").expect("parse failed");
    let Statement::Zone(zone) = &conf.statements[0] else {
        panic!("not a zone statement");
    };
    assert_eq!(non_local_zone_type(zone), None);
}

#[test]
fn test_map_options_without_directory_falls_back_to_conf_parent() {
    let dir = ConfDir::new();
    dir.write("db.example.com", SOA_EXAMPLE_COM);
    let named_conf = format!(
        "options {{ recursion no; }};\n{}",
        primary_zone("db.example.com")
    );

    let manifests = dir.map(&named_conf, default_config());

    assert_eq!(manifests[0]["spec"]["soaRecord"]["serial"], 7);
}

/// CR names carry a per-type index across the whole zone, so a record that
/// sorts earlier renames every later record of that type. A re-import then
/// creates duplicates and orphans the old objects, which keep serving their
/// old data (roadmap 01, stable CR names).
#[test]
#[ignore = "CR names are index-based and shift when a record is added (roadmap 01)"]
fn test_adding_a_record_does_not_rename_others() {
    let dir = ConfDir::new();
    dir.write("db.example.com", SOA_EXAMPLE_COM);
    let before = dir.map(&primary_zone("db.example.com"), default_config());
    dir.write(
        "db.example.com",
        &format!("{SOA_EXAMPLE_COM}aaa IN A 192.0.2.99\n"),
    );

    let after = dir.map(&primary_zone("db.example.com"), default_config());

    let ns1_name = |ms: &[Value]| find(ms, "ARecord", "ns1")["metadata"]["name"].clone();
    assert_eq!(ns1_name(&before), ns1_name(&after));
}
