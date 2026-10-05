// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Hand-written structs for bindy CRD manifests (ADR-0002).
//!
//! These mirror the relevant fields of the bindy CRD spec without importing
//! the full bindy crate. [`ToJson`] renders them with the `camelCase` field
//! names of the Kubernetes wire format, omitting `None` options and empty
//! lists and maps (ADR-0006: replaces the serde derives).

use std::collections::BTreeMap;

use crate::json::Value;

// ── Constants ────────────────────────────────────────────────────────────────

pub const API_VERSION: &str = "bindy.firestoned.io/v1beta1";
pub const LABEL_MANAGED_BY: &str = "bindy.firestoned.io/managed-by";
pub const LABEL_ZONE: &str = "bindy.firestoned.io/zone";
pub const LABEL_SOURCE: &str = "bindy.firestoned.io/source";
pub const MANAGED_BY_VALUE: &str = "forage";
pub const SOURCE_VALUE: &str = "named-conf-import";

// ── Generic Kubernetes object wrapper ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct K8sObject<S> {
    pub api_version: String,
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: S,
}

#[derive(Debug, Clone, Default)]
pub struct ObjectMeta {
    pub name: String,
    pub namespace: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
}

// ── DNSZone ──────────────────────────────────────────────────────────────────

pub type DnsZone = K8sObject<DnsZoneSpec>;

#[derive(Debug, Clone)]
pub struct DnsZoneSpec {
    pub zone_name: String,
    pub soa_record: SoaRecord,
    pub ttl: Option<i32>,
    pub cluster_ref: Option<String>,
    pub name_servers: Vec<NameServer>,
    pub records_from: Vec<RecordSource>,
}

#[derive(Debug, Clone)]
pub struct SoaRecord {
    pub primary_ns: String,
    pub admin_email: String,
    pub serial: i64,
    pub refresh: i32,
    pub retry: i32,
    pub expire: i32,
    pub negative_ttl: i32,
}

#[derive(Debug, Clone)]
pub struct NameServer {
    pub hostname: String,
    pub ipv4_address: Option<String>,
    pub ipv6_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordSource {
    pub selector: LabelSelector,
}

#[derive(Debug, Clone)]
pub struct LabelSelector {
    pub match_labels: BTreeMap<String, String>,
}

// ── ARecord ──────────────────────────────────────────────────────────────────

pub type ARecord = K8sObject<ARecordSpec>;

#[derive(Debug, Clone)]
pub struct ARecordSpec {
    pub name: String,
    pub ipv4_addresses: Vec<String>,
    pub ttl: Option<i32>,
}

// ── AAAARecord ───────────────────────────────────────────────────────────────

pub type AaaaRecord = K8sObject<AaaaRecordSpec>;

#[derive(Debug, Clone)]
pub struct AaaaRecordSpec {
    pub name: String,
    pub ipv6_addresses: Vec<String>,
    pub ttl: Option<i32>,
}

// ── CNAMERecord ──────────────────────────────────────────────────────────────

pub type CnameRecord = K8sObject<CnameRecordSpec>;

#[derive(Debug, Clone)]
pub struct CnameRecordSpec {
    pub name: String,
    /// Canonical name the alias points to (bindy field `target`).
    pub target: String,
    pub ttl: Option<i32>,
}

// ── MXRecord ─────────────────────────────────────────────────────────────────

pub type MxRecord = K8sObject<MxRecordSpec>;

#[derive(Debug, Clone)]
pub struct MxRecordSpec {
    pub name: String,
    /// MX preference (bindy field `priority`).
    pub priority: u16,
    /// Mail exchanger host (bindy field `mailServer`).
    pub mail_server: String,
    pub ttl: Option<i32>,
}

// ── TXTRecord ────────────────────────────────────────────────────────────────

pub type TxtRecord = K8sObject<TxtRecordSpec>;

#[derive(Debug, Clone)]
pub struct TxtRecordSpec {
    pub name: String,
    /// TXT character-strings, one element each, as in the zone file.
    pub text: Vec<String>,
    pub ttl: Option<i32>,
}

// ── SRVRecord ────────────────────────────────────────────────────────────────

pub type SrvRecord = K8sObject<SrvRecordSpec>;

#[derive(Debug, Clone)]
pub struct SrvRecordSpec {
    pub name: String,
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
    pub ttl: Option<i32>,
}

// ── CAARecord ────────────────────────────────────────────────────────────────

pub type CaaRecord = K8sObject<CaaRecordSpec>;

#[derive(Debug, Clone)]
pub struct CaaRecordSpec {
    pub name: String,
    pub flags: u8,
    pub tag: String,
    pub value: String,
    pub ttl: Option<i32>,
}

// ── Wire format ──────────────────────────────────────────────────────────────

/// Renders a manifest struct in the bindy CRD wire format.
pub trait ToJson {
    /// The JSON value, with camelCase keys.
    fn to_json(&self) -> Value;
}

impl<S: ToJson> ToJson for K8sObject<S> {
    fn to_json(&self) -> Value {
        Value::object()
            .with("apiVersion", self.api_version.as_str())
            .with("kind", self.kind.as_str())
            .with("metadata", self.metadata.to_json())
            .with("spec", self.spec.to_json())
    }
}

impl ToJson for ObjectMeta {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("namespace", self.namespace.as_str())
            .with_nonempty("labels", self.labels.clone().into())
            .with_nonempty("annotations", self.annotations.clone().into())
    }
}

impl ToJson for DnsZoneSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("zoneName", self.zone_name.as_str())
            .with("soaRecord", self.soa_record.to_json())
            .with_opt("ttl", self.ttl)
            .with_opt("clusterRef", self.cluster_ref.clone())
            .with_nonempty(
                "nameServers",
                self.name_servers
                    .iter()
                    .map(ToJson::to_json)
                    .collect::<Vec<_>>()
                    .into(),
            )
            .with_nonempty(
                "recordsFrom",
                self.records_from
                    .iter()
                    .map(ToJson::to_json)
                    .collect::<Vec<_>>()
                    .into(),
            )
    }
}

impl ToJson for SoaRecord {
    fn to_json(&self) -> Value {
        Value::object()
            .with("primaryNs", self.primary_ns.as_str())
            .with("adminEmail", self.admin_email.as_str())
            .with("serial", self.serial)
            .with("refresh", self.refresh)
            .with("retry", self.retry)
            .with("expire", self.expire)
            .with("negativeTtl", self.negative_ttl)
    }
}

impl ToJson for NameServer {
    fn to_json(&self) -> Value {
        Value::object()
            .with("hostname", self.hostname.as_str())
            .with_opt("ipv4Address", self.ipv4_address.clone())
            .with_opt("ipv6Address", self.ipv6_address.clone())
    }
}

impl ToJson for RecordSource {
    fn to_json(&self) -> Value {
        Value::object().with("selector", self.selector.to_json())
    }
}

impl ToJson for LabelSelector {
    fn to_json(&self) -> Value {
        Value::object().with_nonempty("matchLabels", self.match_labels.clone().into())
    }
}

impl ToJson for ARecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("ipv4Addresses", self.ipv4_addresses.clone())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for AaaaRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("ipv6Addresses", self.ipv6_addresses.clone())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for CnameRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("target", self.target.as_str())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for MxRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("priority", self.priority)
            .with("mailServer", self.mail_server.as_str())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for TxtRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("text", self.text.clone())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for SrvRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("priority", self.priority)
            .with("weight", self.weight)
            .with("port", self.port)
            .with("target", self.target.as_str())
            .with_opt("ttl", self.ttl)
    }
}

impl ToJson for CaaRecordSpec {
    fn to_json(&self) -> Value {
        Value::object()
            .with("name", self.name.as_str())
            .with("flags", self.flags)
            .with("tag", self.tag.as_str())
            .with("value", self.value.as_str())
            .with_opt("ttl", self.ttl)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "crd_tests.rs"]
mod crd_tests;
