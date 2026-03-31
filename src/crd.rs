// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Minimal serde-serializable structs for bindy CRD manifests.
//!
//! These mirror the relevant fields of the bindy CRD spec without importing
//! the full bindy crate. Field names use `camelCase` to match the Kubernetes
//! API wire format.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ── Constants ────────────────────────────────────────────────────────────────

pub const API_VERSION: &str = "bindy.firestoned.io/v1beta1";
pub const LABEL_MANAGED_BY: &str = "bindy.firestoned.io/managed-by";
pub const LABEL_ZONE: &str = "bindy.firestoned.io/zone";
pub const LABEL_SOURCE: &str = "bindy.firestoned.io/source";
pub const MANAGED_BY_VALUE: &str = "forage";
pub const SOURCE_VALUE: &str = "named-conf-import";

// ── Generic Kubernetes object wrapper ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct K8sObject<S> {
    pub api_version: String,
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: S,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMeta {
    pub name: String,
    pub namespace: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

// ── DNSZone ──────────────────────────────────────────────────────────────────

pub type DnsZone = K8sObject<DnsZoneSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsZoneSpec {
    pub zone_name: String,
    pub soa_record: SoaRecord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_ref: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub name_servers: Vec<NameServer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub records_from: Vec<RecordSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoaRecord {
    pub primary_ns: String,
    pub admin_email: String,
    pub serial: i64,
    pub refresh: i32,
    pub retry: i32,
    pub expire: i32,
    pub negative_ttl: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameServer {
    pub hostname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSource {
    pub selector: LabelSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub match_labels: BTreeMap<String, String>,
}

// ── ARecord ──────────────────────────────────────────────────────────────────

pub type ARecord = K8sObject<ARecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ARecordSpec {
    pub name: String,
    pub ipv4_addresses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── AAAARecord ───────────────────────────────────────────────────────────────

pub type AaaaRecord = K8sObject<AaaaRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AaaaRecordSpec {
    pub name: String,
    pub ipv6_addresses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── CNAMERecord ──────────────────────────────────────────────────────────────

pub type CnameRecord = K8sObject<CnameRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CnameRecordSpec {
    pub name: String,
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── MXRecord ─────────────────────────────────────────────────────────────────

pub type MxRecord = K8sObject<MxRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MxRecordSpec {
    pub name: String,
    pub preference: u16,
    pub exchange: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── TXTRecord ────────────────────────────────────────────────────────────────

pub type TxtRecord = K8sObject<TxtRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxtRecordSpec {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── SRVRecord ────────────────────────────────────────────────────────────────

pub type SrvRecord = K8sObject<SrvRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SrvRecordSpec {
    pub name: String,
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}

// ── CAARecord ────────────────────────────────────────────────────────────────

pub type CaaRecord = K8sObject<CaaRecordSpec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaaRecordSpec {
    pub name: String,
    pub flags: u8,
    pub tag: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i32>,
}
