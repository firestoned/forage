// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use hornet_bind9::parse_named_conf;

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
    let rr = hornet_bind9::ast::zone_file::ResourceRecord {
        name: Some(hornet_bind9::ast::zone_file::Name::new("@")),
        ttl: None,
        class: None,
        rdata: hornet_bind9::ast::zone_file::RData::A("1.2.3.4".parse().unwrap()),
    };
    assert_eq!(resolve_record_name(&rr, "example.com"), "@");
}

#[test]
fn test_resolve_record_name_relative() {
    let rr = hornet_bind9::ast::zone_file::ResourceRecord {
        name: Some(hornet_bind9::ast::zone_file::Name::new("www")),
        ttl: None,
        class: None,
        rdata: hornet_bind9::ast::zone_file::RData::A("1.2.3.4".parse().unwrap()),
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
    let manifests = mapper.map(&conf).expect("map failed");

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
    let manifests = mapper.map(&conf).expect("map failed");

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
    let manifests = mapper.map(&conf).expect("map failed");

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
    let manifests = mapper.map(&conf).expect("map failed");

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
    let manifests = mapper.map(&conf).expect("map failed");

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
    let manifests = mapper.map(&conf).expect("map failed");

    let selector = &manifests[0]["spec"]["recordsFrom"][0]["selector"]["matchLabels"];
    assert_eq!(selector[LABEL_ZONE], "example.com");
}
