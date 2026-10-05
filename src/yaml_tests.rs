// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::json::Value;

    #[test]
    fn test_block_layout_matches_serde_yaml() {
        let v = Value::object().with("kind", "DNSZone").with(
            "spec",
            Value::object()
                .with(
                    "recordsFrom",
                    vec![Value::object().with(
                        "selector",
                        Value::object()
                            .with("matchLabels", Value::object().with("zone", "example.com")),
                    )],
                )
                .with("text", vec!["one", "two"])
                .with("ttl", 300_i64),
        );
        assert_eq!(
            to_yaml(&v),
            "kind: DNSZone\nspec:\n  recordsFrom:\n  - selector:\n      matchLabels:\n        zone: example.com\n  text:\n  - one\n  - two\n  ttl: 300\n"
        );
    }

    #[test]
    fn test_empty_containers_bool_null_and_nested_sequences() {
        let v = Value::object()
            .with("a", Value::Array(vec![]))
            .with("b", true)
            .with("m", Value::object())
            .with("n", Value::Null)
            .with(
                "s",
                vec![Value::Array(vec![Value::from("x"), Value::from("y")])],
            );
        assert_eq!(
            to_yaml(&v),
            "a: []\nb: true\nm: {}\nn: null\ns:\n- - x\n  - y\n"
        );
    }

    #[test]
    fn test_top_level_scalar_and_sequence() {
        assert_eq!(to_yaml(&Value::from("plain")), "plain\n");
        assert_eq!(to_yaml(&Value::Array(vec![Value::from(1_i64)])), "- 1\n");
    }

    #[test]
    fn test_scalars_that_stay_plain() {
        for s in [
            "www",
            "ns1.example.com.",
            "192.0.2.1",
            "2001:db8::10",
            "v=spf1 mx -all",
            "-all",
            "a:b",
            "a#b",
            "letsencrypt.org",
            "x,y",
            "a?b",
            "é",
            ":x",
            "0b2",
        ] {
            assert_eq!(scalar(s), s, "{s}");
        }
    }

    #[test]
    fn test_scalars_resolving_to_other_types_are_single_quoted() {
        for s in [
            "", "~", "null", "Null", "NULL", "true", "False", "TRUE", "300", "-12", "+5", "0x1F",
            "0o17", "-0x1F", "-0o17", "+0x1", "+0o7", "0b101", "0", "1.5", "+1e3", ".5", ".inf",
            "-.Inf", ".NaN", "007",
        ] {
            assert_eq!(scalar(s), format!("'{s}'"), "{s}");
        }
    }

    #[test]
    fn test_plain_lookalikes_that_are_not_numbers() {
        for s in [
            "0x", "0xZZ", "0o9", "+-1", "-0xZZ", "-0o9", "inf", "nan", "0x+1", "+.inf.x",
        ] {
            assert_eq!(scalar(s), s, "{s}");
        }
    }

    #[test]
    fn test_indicators_force_single_quotes() {
        for (s, q) in [
            ("@", "'@'"),
            ("*.com", "'*.com'"),
            ("- x", "'- x'"),
            ("-", "'-'"),
            ("key: v", "'key: v'"),
            ("end:", "'end:'"),
            ("a #b", "'a #b'"),
            ("? x", "'? x'"),
            (" lead", "' lead'"),
            ("trail ", "'trail '"),
            ("it's: x", "'it''s: x'"),
            ("---", "'---'"),
            ("...", "'...'"),
        ] {
            assert_eq!(scalar(s), q, "{s}");
        }
    }

    #[test]
    fn test_special_characters_force_double_quotes() {
        assert_eq!(scalar("a\tb"), "\"a\\tb\"");
        assert_eq!(scalar("l1\nl2"), "\"l1\\nl2\"");
        assert_eq!(scalar("q\"\\\u{1}\r"), "\"q\\\"\\\\\\x01\\r\"");
    }
}
