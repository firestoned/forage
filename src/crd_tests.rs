// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Wire-format tests: the JSON keys each spec serializes to must be the field
//! names in bindy's CRD schema (ADR-0002). Field names were checked against
//! bindy v0.7.1 `deploy/operator/crds/*.crd.yaml`; the e2e `schema` suite
//! re-checks them against a live API server.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::json::Value;

    fn spec_keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("spec serializes to an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    #[test]
    fn test_cname_spec_serializes_target_not_alias() {
        let spec = CnameRecordSpec {
            name: "docs".into(),
            target: "www.example.com.".into(),
            ttl: None,
        };

        let value = spec.to_json();

        assert_eq!(spec_keys(&value), vec!["name", "target"]);
        assert_eq!(value["target"], Value::from("www.example.com."));
    }

    #[test]
    fn test_mx_spec_serializes_priority_and_mail_server() {
        let spec = MxRecordSpec {
            name: "@".into(),
            priority: 10,
            mail_server: "mail.example.com.".into(),
            ttl: Some(300),
        };

        let value = spec.to_json();

        assert_eq!(
            spec_keys(&value),
            vec!["mailServer", "name", "priority", "ttl"]
        );
        assert_eq!(value["priority"], Value::from(10));
        assert_eq!(value["mailServer"], Value::from("mail.example.com."));
    }

    #[test]
    fn test_txt_spec_serializes_text_as_string_array() {
        let spec = TxtRecordSpec {
            name: "@".into(),
            text: vec!["v=spf1 mx".into(), " -all".into()],
            ttl: None,
        };

        let value = spec.to_json();

        assert_eq!(spec_keys(&value), vec!["name", "text"]);
        assert_eq!(value["text"], Value::from(vec!["v=spf1 mx", " -all"]));
    }

    #[test]
    fn test_srv_spec_field_names_match_bindy() {
        let spec = SrvRecordSpec {
            name: "_sip._tcp".into(),
            priority: 10,
            weight: 60,
            port: 5060,
            target: "sip.example.com.".into(),
            ttl: None,
        };

        let value = spec.to_json();

        assert_eq!(
            spec_keys(&value),
            vec!["name", "port", "priority", "target", "weight"]
        );
    }

    #[test]
    fn test_caa_spec_field_names_match_bindy() {
        let spec = CaaRecordSpec {
            name: "@".into(),
            flags: 0,
            tag: "issue".into(),
            value: "letsencrypt.org".into(),
            ttl: None,
        };

        let value = spec.to_json();

        assert_eq!(spec_keys(&value), vec!["flags", "name", "tag", "value"]);
    }

    #[test]
    fn test_ttl_omitted_when_none() {
        let spec = ARecordSpec {
            name: "www".into(),
            ipv4_addresses: vec!["192.0.2.10".into()],
            ttl: None,
        };

        let value = spec.to_json();

        assert_eq!(spec_keys(&value), vec!["ipv4Addresses", "name"]);
    }
}
