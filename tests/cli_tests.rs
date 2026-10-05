// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Black-box tests: run the built `forage` binary against the fixtures in
//! `tests/fixtures/` and assert on the manifest stream it prints. The e2e
//! suites (`tests/e2e/forage-e2e.sh`) feed the same fixture to a real API
//! server; these tests pin the content without needing a cluster.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::{parse_stream, strings, Json, TempDir};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .join("named.conf")
}

fn forage() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forage"))
}

/// Runs forage on `conf` with extra args and `--output json`, asserts it
/// succeeded, and parses the concatenated JSON stream.
fn run_json(conf: &Path, extra: &[&str]) -> Vec<Json> {
    let output = forage()
        .arg("--conf")
        .arg(conf)
        .args(["--namespace", "forage-test", "--output", "json"])
        .args(extra)
        .output()
        .expect("forage runs");
    assert!(
        output.status.success(),
        "forage exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json_stream(&output.stdout)
}

fn parse_json_stream(stdout: &[u8]) -> Vec<Json> {
    parse_stream(&String::from_utf8_lossy(stdout))
}

/// Runs forage with `--output json` on a named fixture.
fn run_forage(fixture_name: &str) -> Vec<Json> {
    run_json(&fixture(fixture_name), &[])
}

/// Writes a one-zone `named.conf` for `example.com` plus its zone file into a
/// temp dir and returns the dir (kept alive by the caller) and conf path.
fn temp_zone(zone_file: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new();
    dir.write("db.example.com", zone_file);
    let conf = dir.write(
        "named.conf",
        "zone \"example.com\" { type primary; file \"db.example.com\"; };\n",
    );
    (dir, conf)
}

fn kind_counts(manifests: &[Json]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for m in manifests {
        let kind = m["kind"].as_str().expect("manifest has a kind").to_string();
        *counts.entry(kind).or_insert(0) += 1;
    }
    counts
}

fn only_of_kind<'a>(manifests: &'a [Json], kind: &str) -> &'a Json {
    let mut matching = manifests.iter().filter(|m| m["kind"] == kind);
    let first = matching
        .next()
        .unwrap_or_else(|| panic!("no {kind} emitted"));
    assert!(matching.next().is_none(), "more than one {kind} emitted");
    first
}

#[test]
fn test_basic_fixture_emits_every_supported_kind() {
    let manifests = run_forage("basic");

    let expected: BTreeMap<String, usize> = [
        ("DNSZone", 2),
        ("ARecord", 6),
        ("AAAARecord", 1),
        ("CNAMERecord", 1),
        ("MXRecord", 1),
        ("TXTRecord", 1),
        ("SRVRecord", 1),
        ("CAARecord", 1),
    ]
    .into_iter()
    .map(|(k, n)| (k.to_string(), n))
    .collect();
    assert_eq!(kind_counts(&manifests), expected);
}

#[test]
fn test_basic_fixture_dns_zones_come_first() {
    let manifests = run_forage("basic");

    let zones = manifests
        .iter()
        .take_while(|m| m["kind"] == "DNSZone")
        .count();
    assert_eq!(zones, 2);
}

#[test]
fn test_basic_fixture_collapses_multi_address_a_records() {
    let manifests = run_forage("basic");

    let www = manifests
        .iter()
        .find(|m| m["kind"] == "ARecord" && m["spec"]["name"] == "www")
        .expect("www ARecord emitted");
    assert_eq!(
        www["spec"]["ipv4Addresses"],
        strings(&["198.51.100.10", "198.51.100.11"])
    );
}

#[test]
fn test_basic_fixture_record_specs_use_bindy_field_names() {
    let manifests = run_forage("basic");

    let cname = only_of_kind(&manifests, "CNAMERecord");
    assert_eq!(cname["spec"]["target"], "www.example.com.");

    let mx = only_of_kind(&manifests, "MXRecord");
    assert_eq!(mx["spec"]["priority"], 10_i64);
    assert_eq!(mx["spec"]["mailServer"], "mail.example.com.");

    let txt = only_of_kind(&manifests, "TXTRecord");
    assert_eq!(txt["spec"]["text"], strings(&["v=spf1 mx -all"]));
}

#[test]
fn test_basic_fixture_secondary_zone_is_a_shell() {
    let manifests = run_forage("basic");

    let org = manifests
        .iter()
        .find(|m| m["kind"] == "DNSZone" && m["spec"]["zoneName"] == "example.org")
        .expect("example.org DNSZone emitted");
    assert!(org["spec"].get("nameServers").is_none());
}

#[test]
fn test_output_is_deterministic() {
    assert_eq!(run_forage("basic"), run_forage("basic"));
}

// ── CLI surface ──────────────────────────────────────────────────────────────

#[test]
fn test_default_output_is_a_yaml_stream() {
    let output = forage()
        .arg("--conf")
        .arg(fixture("basic"))
        .output()
        .expect("forage runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(stdout.starts_with("---\napiVersion: bindy.firestoned.io/v1beta1\n"));
    assert_eq!(stdout.matches("\n---\n").count() + 1, 14);
    assert!(stdout.contains("namespace: bindy-system"));
}

/// The rendered stream is pinned byte for byte. The golden files were
/// captured from the serde-based implementation before ADR-0006 replaced it.
#[test]
fn test_outputs_match_golden_files() {
    for name in ["basic", "edge"] {
        for format in ["yaml", "json"] {
            let output = forage()
                .arg("--conf")
                .arg(fixture(name))
                .args(["--namespace", "forage-test", "--output", format])
                .output()
                .expect("forage runs");
            let golden =
                std::fs::read_to_string(fixture(name).with_file_name(format!("expected.{format}")))
                    .expect("golden file");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                golden,
                "{name} {format}"
            );
        }
    }
}

#[test]
fn test_missing_conf_fails_with_context() {
    let output = forage()
        .args(["--conf", "/nonexistent/forage/named.conf"])
        .output()
        .expect("forage runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to parse /nonexistent/forage/named.conf"),
        "stderr: {stderr}"
    );
    assert_eq!(output.stdout, b"");
}

#[test]
fn test_unparseable_named_conf_fails_with_file_and_line() {
    let dir = TempDir::new();
    let conf = dir.write(
        "named.conf",
        "options { directory \"/x\";\nzone \"a\" { type primary; };\n",
    );

    let output = forage()
        .arg("--conf")
        .arg(&conf)
        .output()
        .expect("forage runs");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("named.conf: line 1: unclosed '{'"),
        "stderr: {stderr}"
    );
    assert_eq!(output.stdout, b"");
}

#[test]
fn test_help_flag_prints_usage() {
    let output = forage().arg("--help").output().expect("forage runs");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: forage [OPTIONS]"), "{stdout}");
    assert!(stdout.contains("--record-types <RECORD_TYPES>"), "{stdout}");
}

#[test]
fn test_usage_error_exits_2() {
    let output = forage().arg("--bogus").output().expect("forage runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument '--bogus'"));
}

#[test]
fn test_unparseable_zone_line_is_reported_not_dropped_silently() {
    let (_dir, conf) = temp_zone(
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\nwww IN A 192.0.2.1\nbad IN A 999.0.0.1\n",
    );

    let output = forage()
        .arg("--conf")
        .arg(&conf)
        .args(["--output", "json"])
        .env_remove("RUST_LOG")
        .output()
        .expect("forage runs");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("db.example.com:3: not imported, could not parse: bad IN A 999.0.0.1"),
        "stderr: {stderr}"
    );
    assert_eq!(
        kind_counts(&parse_json_stream(&output.stdout)).get("ARecord"),
        Some(&1)
    );
}

#[test]
fn test_invalid_output_format_is_rejected() {
    let output = forage()
        .arg("--conf")
        .arg(fixture("basic"))
        .args(["--output", "toml"])
        .output()
        .expect("forage runs");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("toml"));
}

#[test]
fn test_debug_flag_logs_to_stderr_only() {
    let output = forage()
        .arg("--conf")
        .arg(fixture("basic"))
        .args(["--output", "json", "--debug"])
        .env_remove("RUST_LOG")
        .output()
        .expect("forage runs");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DEBUG"), "stderr: {stderr}");
    assert!(stderr.contains("zone file base directory"));
    assert_eq!(parse_json_stream(&output.stdout).len(), 14);
}

#[test]
fn test_default_log_level_is_warn() {
    let output = forage()
        .arg("--conf")
        .arg(fixture("basic"))
        .env_remove("RUST_LOG")
        .output()
        .expect("forage runs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("WARN"), "stderr: {stderr}");
    assert!(!stderr.contains("INFO"), "stderr: {stderr}");
}

#[test]
fn test_rust_log_env_raises_verbosity() {
    let output = forage()
        .arg("--conf")
        .arg(fixture("basic"))
        .env("RUST_LOG", "info")
        .output()
        .expect("forage runs");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("emitted 14 manifest(s)"),
        "stderr: {stderr}"
    );
}

#[test]
fn test_record_types_flag_is_case_and_space_insensitive() {
    let manifests = run_json(&fixture("basic"), &["--record-types", " mx , Txt"]);

    let expected: BTreeMap<String, usize> = [("DNSZone", 2), ("MXRecord", 1), ("TXTRecord", 1)]
        .into_iter()
        .map(|(k, n)| (k.to_string(), n))
        .collect();
    assert_eq!(kind_counts(&manifests), expected);
}

#[test]
fn test_skip_records_flag_emits_only_zones() {
    let manifests = run_json(&fixture("basic"), &["--skip-records"]);

    assert!(manifests.iter().all(|m| m["kind"] == "DNSZone"));
    assert_eq!(manifests.len(), 2);
}

#[test]
fn test_zone_filter_flag() {
    let manifests = run_json(&fixture("basic"), &["--zone-filter", "*.org"]);

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0]["spec"]["zoneName"], "example.org");
}

#[test]
fn test_cluster_ref_and_namespace_flags() {
    let manifests = run_json(&fixture("basic"), &["--cluster-ref", "prod-dns"]);

    assert!(manifests
        .iter()
        .all(|m| m["metadata"]["namespace"] == "forage-test"));
    assert!(manifests
        .iter()
        .filter(|m| m["kind"] == "DNSZone")
        .all(|m| m["spec"]["clusterRef"] == "prod-dns"));
}

#[test]
fn test_zone_dir_flag_redirects_zone_file_lookup() {
    let (dir, _) = temp_zone("");
    let conf = dir.path().join("named.conf");

    let manifests = run_json(
        &conf,
        &[
            "--zone-dir",
            fixture("basic")
                .parent()
                .and_then(Path::to_str)
                .expect("path"),
        ],
    );

    assert_eq!(kind_counts(&manifests).get("ARecord"), Some(&6));
}

#[test]
fn test_every_manifest_is_labelled_managed_by_forage() {
    let manifests = run_forage("basic");

    assert!(manifests.iter().all(|m| {
        m["metadata"]["labels"]["bindy.firestoned.io/managed-by"] == "forage"
            && m["metadata"]["labels"]["bindy.firestoned.io/source"] == "named-conf-import"
    }));
    assert!(manifests
        .iter()
        .filter(|m| m["kind"] != "DNSZone")
        .all(|m| m["metadata"]["labels"]["bindy.firestoned.io/zone"] == "example.com"));
}

#[test]
fn test_version_flag() {
    let output = forage().arg("--version").output().expect("forage runs");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("forage {}", env!("CARGO_PKG_VERSION"))
    );
}

// ── Zone-file parse regressions ──────────────────────────────────────────────
// hornet-bind9 0.1 lost or corrupted these records (roadmap 01). forage's own
// parser (ADR-0006) handles them; these keep it that way.

/// An SRV record directly after a quoted TXT record never reaches the mapper.
#[test]
fn test_srv_after_txt_is_not_dropped() {
    let manifests = run_forage("srv-after-txt");

    assert_eq!(kind_counts(&manifests).get("SRVRecord"), Some(&1));
}

/// Any record after a quoted TXT is lost, not only SRV.
#[test]
fn test_record_after_txt_is_not_dropped() {
    let (_dir, conf) = temp_zone(
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\n@ IN TXT \"x\"\nwww IN A 192.0.2.2\n",
    );

    let manifests = run_json(&conf, &[]);

    assert_eq!(kind_counts(&manifests).get("ARecord"), Some(&1));
}

/// The record after a one-line SOA is lost (here: the zone's NS).
#[test]
fn test_record_after_one_line_soa_is_not_dropped() {
    let (_dir, conf) = temp_zone(
        "@ IN SOA ns1.example.com. h.example.com. 1 2 3 4 5\n@ IN NS ns1.example.com.\nns1 IN A 192.0.2.1\n",
    );

    let manifests = run_json(&conf, &[]);

    assert_eq!(
        manifests[0]["spec"]["nameServers"][0]["hostname"],
        "ns1.example.com."
    );
}

/// A blank owner (inherit the previous name) is misread: the class becomes the
/// owner, producing a record named `IN`.
#[test]
fn test_inherited_owner_is_not_named_in() {
    let (_dir, conf) = temp_zone(
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\nwww IN A 192.0.2.1\n    IN TXT \"x\"\n",
    );

    let manifests = run_json(&conf, &[]);

    let txt = only_of_kind(&manifests, "TXTRecord");
    assert_ne!(txt["spec"]["name"], "IN");
}

// ── Diagnostics on stderr ────────────────────────────────────────────────────

#[test]
fn test_missing_zone_file_warns_and_emits_zone_shell() {
    let dir = TempDir::new();
    let conf = dir.path().join("named.conf");
    std::fs::write(
        &conf,
        "zone \"example.com\" { type primary; file \"db.missing\"; };\n",
    )
    .expect("write conf");

    let output = forage()
        .arg("--conf")
        .arg(&conf)
        .args(["--output", "json"])
        .env_remove("RUST_LOG")
        .output()
        .expect("forage runs");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to parse zone file") && stderr.contains("db.missing"),
        "stderr: {stderr}"
    );
    let manifests = parse_json_stream(&output.stdout);
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0]["kind"], "DNSZone");
}

#[test]
fn test_unsupported_record_type_is_logged_at_debug() {
    let (_dir, conf) = temp_zone(
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\n10 IN PTR host.example.com.\n",
    );

    let output = forage()
        .arg("--conf")
        .arg(&conf)
        .args(["--output", "json", "--debug"])
        .output()
        .expect("forage runs");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("skipping unsupported record type 'PTR' for name '10'"),
        "stderr: {stderr}"
    );
    assert_eq!(parse_json_stream(&output.stdout).len(), 1);
}

// ── forage regressions (roadmap 01) ──────────────────────────────────────────

/// Zones declared in an `include`d file are never imported: hornet returns the
/// include as a statement and forage does not follow it, so the run succeeds
/// with zero manifests.
#[test]
#[ignore = "forage does not follow named.conf include statements (roadmap 01)"]
fn test_zones_in_included_files_are_imported() {
    let dir = TempDir::new();
    std::fs::write(
        dir.path().join("zones.conf"),
        "zone \"example.com\" { type primary; file \"db.example.com\"; };\n",
    )
    .expect("write include");
    std::fs::write(
        dir.path().join("db.example.com"),
        "@ IN SOA ns1.example.com. h.example.com. ( 1 2 3 4 5 )\nwww IN A 192.0.2.7\n",
    )
    .expect("write zone");
    let conf = dir.path().join("named.conf");
    std::fs::write(&conf, "include \"zones.conf\";\n").expect("write conf");

    let manifests = run_json(&conf, &[]);

    assert_eq!(kind_counts(&manifests).get("DNSZone"), Some(&1));
}
