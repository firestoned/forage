// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::collections::BTreeMap;

    fn sample() -> Value {
        Value::object()
            .with("name", "www")
            .with("ttl", 300_i64)
            .with("tags", vec!["a", "b"])
            .with("nested", Value::object().with("on", true))
    }

    #[test]
    fn test_pretty_matches_serde_json_layout() {
        assert_eq!(
            sample().to_json_pretty(),
            "{\n  \"name\": \"www\",\n  \"nested\": {\n    \"on\": true\n  },\n  \"tags\": [\n    \"a\",\n    \"b\"\n  ],\n  \"ttl\": 300\n}"
        );
    }

    #[test]
    fn test_empty_containers_and_null() {
        let v = Value::object()
            .with("a", Value::Array(vec![]))
            .with("o", Value::object())
            .with("n", Value::Null);
        assert_eq!(
            v.to_json_pretty(),
            "{\n  \"a\": [],\n  \"n\": null,\n  \"o\": {}\n}"
        );
    }

    #[test]
    fn test_string_escapes() {
        let v = Value::from("q\"b\\n\nr\rt\tb\u{8}f\u{c}c\u{1}é");
        assert_eq!(
            v.to_json_pretty(),
            "\"q\\\"b\\\\n\\nr\\rt\\tb\\bf\\fc\\u0001é\""
        );
    }

    #[test]
    fn test_with_opt_and_with_nonempty_skip() {
        let v = Value::object()
            .with_opt("ttl", None::<i64>)
            .with_opt("kept", Some(1_i64))
            .with_nonempty("list", Value::Array(vec![]))
            .with_nonempty("map", Value::object())
            .with_nonempty("scalar", Value::from("kept"));
        assert_eq!(
            v.to_json_pretty(),
            "{\n  \"kept\": 1,\n  \"scalar\": \"kept\"\n}"
        );
    }

    #[test]
    fn test_indexing_and_accessors() {
        let v = sample();
        assert_eq!(v["name"], "www");
        assert_eq!(v["ttl"], 300);
        assert_eq!(v["tags"][1], "b");
        assert_eq!(v["missing"]["deeper"], Value::Null);
        assert_eq!(v["tags"][9], Value::Null);
        assert_eq!(v["name"][0], Value::Null);
        assert_eq!(v["name"].as_str(), Some("www"));
        assert_eq!(v["ttl"].as_str(), None);
        assert_eq!(v["ttl"].as_i64(), Some(300));
        assert_eq!(v["name"].as_i64(), None);
        assert!(v.get("name").is_some());
        assert!(v.get("nope").is_none());
        assert!(v["name"].get("x").is_none());
        assert_eq!(v["tags"].as_array().map(Vec::len), Some(2));
        assert!(v["name"].as_array().is_none());
        assert_eq!(v.as_object().map(BTreeMap::len), Some(4));
        assert!(v["name"].as_object().is_none());
    }

    #[test]
    fn test_conversions() {
        assert_eq!(Value::from(String::from("s")), "s");
        assert_eq!(Value::from(7_u8), 7);
        assert_eq!(Value::from(7_u16), 7_i64);
        assert_eq!(Value::from(7_u32), 7);
        assert_eq!(Value::from(-7_i32), -7);
        assert_eq!(Value::from(false), Value::Bool(false));
        let labels = BTreeMap::from([("k".to_string(), "v".to_string())]);
        assert_eq!(Value::from(labels)["k"], "v");
        assert_ne!(Value::from("a"), "b");
        assert_ne!(Value::from(1_i64), "1");
        assert_ne!(Value::from("1"), 1);
    }

    #[test]
    fn test_insert_on_non_object_is_ignored() {
        let mut v = Value::from("scalar");
        v.insert("k", Value::Null);
        assert_eq!(v, "scalar");
    }
}
