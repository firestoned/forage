// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn records(input: &str) -> Vec<ResourceRecord> {
        parse_zone_file(input).records().cloned().collect()
    }

    fn owners(input: &str) -> Vec<String> {
        records(input)
            .iter()
            .map(|r| {
                r.name
                    .as_ref()
                    .map_or("<none>".into(), |n| n.as_str().to_string())
            })
            .collect()
    }

    #[test]
    fn test_every_supported_type() {
        let rs = records(concat!(
            "@ 3600 IN SOA ns1.example.com. h.example.com. ( 1 2h 3m 4d 1w )\n",
            "@ IN NS ns1\n",
            "www A 192.0.2.1\n",
            "www AAAA 2001:db8::1\n",
            "docs CNAME www\n",
            "@ MX 10 mail\n",
            "@ TXT \"a b\" c \"q\\\"x\\\\y\"\n",
            "_s._tcp SRV 1 2 3 t.example.com.\n",
            "@ CAA 0 issue \"letsencrypt.org\"\n",
            "@ caa 128 iodef mailto:x@example.com\n",
            "1 PTR host.\n",
            "x HINFO cpu os\n",
        ));
        let rdata: Vec<&RData> = rs.iter().map(|r| &r.rdata).collect();
        assert_eq!(
            rdata,
            vec![
                &RData::Soa(SoaData {
                    mname: Name::new("ns1.example.com."),
                    rname: Name::new("h.example.com."),
                    serial: 1,
                    refresh: 7200,
                    retry: 180,
                    expire: 345_600,
                    minimum: 604_800,
                }),
                &RData::Ns(Name::new("ns1")),
                &RData::A(Ipv4Addr::new(192, 0, 2, 1)),
                &RData::Aaaa("2001:db8::1".parse::<Ipv6Addr>().unwrap()),
                &RData::Cname(Name::new("www")),
                &RData::Mx(MxData {
                    preference: 10,
                    exchange: Name::new("mail")
                }),
                &RData::Txt(vec!["a b".into(), "c".into(), "q\"x\\y".into()]),
                &RData::Srv(SrvData {
                    priority: 1,
                    weight: 2,
                    port: 3,
                    target: Name::new("t.example.com.")
                }),
                &RData::Caa(CaaData {
                    flags: 0,
                    tag: "issue".into(),
                    value: "letsencrypt.org".into()
                }),
                &RData::Caa(CaaData {
                    flags: 128,
                    tag: "iodef".into(),
                    value: "mailto:x@example.com".into()
                }),
                &RData::Other("PTR".into()),
                &RData::Other("HINFO".into()),
            ]
        );
        let types: Vec<&str> = rdata.iter().map(|r| r.rtype()).collect();
        assert_eq!(
            types,
            vec![
                "SOA", "NS", "A", "AAAA", "CNAME", "MX", "TXT", "SRV", "CAA", "CAA", "PTR", "HINFO"
            ]
        );
        assert_eq!(rs[0].ttl, Some(3600));
        assert_eq!(rs[0].class.as_deref(), Some("IN"));
        assert_eq!(rs[2].ttl, None);
        assert_eq!(rs[2].class, None);
    }

    #[test]
    fn test_ttl_and_class_in_either_order() {
        let rs = records("a 300 IN A 192.0.2.1\nb IN 1h30m A 192.0.2.2\nc CH A 192.0.2.3\n");
        assert_eq!(
            rs.iter().map(|r| r.ttl).collect::<Vec<_>>(),
            vec![Some(300), Some(5400), None]
        );
        assert_eq!(rs[2].class.as_deref(), Some("CH"));
    }

    #[test]
    fn test_record_after_quoted_txt_is_kept() {
        assert_eq!(
            owners("@ TXT \"x\"\nwww A 192.0.2.2\n_s._tcp SRV 1 2 3 t.\n"),
            vec!["@", "www", "_s._tcp"]
        );
    }

    #[test]
    fn test_record_after_one_line_soa_is_kept() {
        let rs = records("@ IN SOA ns1. h. 1 2 3 4 5\n@ IN NS ns1.example.com.\n");
        assert_eq!(rs.len(), 2);
        assert_eq!(rs[1].rdata, RData::Ns(Name::new("ns1.example.com.")));
    }

    #[test]
    fn test_blank_owner_inherits_previous_owner() {
        assert_eq!(
            owners("  IN A 192.0.2.1\nwww A 192.0.2.2\n    IN TXT \"x\"\n\tMX 5 m.\n"),
            vec!["<none>", "www", "www", "www"]
        );
    }

    #[test]
    fn test_comments_parens_and_quotes() {
        let rs = records(concat!(
            "; full-line comment\n",
            "\n",
            "@ IN SOA ns. h. (\n",
            "    7   ; serial\n",
            "    1 2 3 4 ) ; trailing\n",
            "t TXT \"semi; (paren) kept\" ; comment\n",
        ));
        assert_eq!(rs.len(), 2);
        assert!(matches!(&rs[0].rdata, RData::Soa(soa) if soa.serial == 7 && soa.minimum == 4));
        assert_eq!(rs[1].rdata, RData::Txt(vec!["semi; (paren) kept".into()]));
    }

    #[test]
    fn test_directives() {
        let zf = parse_zone_file("$TTL 1d\n$ORIGIN example.com.\n$INCLUDE other.zone\n$GENERATE 1-2 h$ A 192.0.2.$\nwww A 192.0.2.1\n");
        assert_eq!(
            zf.entries,
            vec![
                Entry::Ttl(86_400),
                Entry::Origin("example.com.".into()),
                Entry::Include("other.zone".into()),
                Entry::Record(ResourceRecord {
                    name: Some(Name::new("www")),
                    ttl: None,
                    class: None,
                    rdata: RData::A(Ipv4Addr::new(192, 0, 2, 1))
                }),
            ]
        );
        assert_eq!(
            zf.skipped,
            vec![SkippedLine {
                line: 4,
                text: "$GENERATE 1-2 h$ A 192.0.2.$".into()
            }]
        );
    }

    #[test]
    fn test_malformed_lines_are_reported_with_their_line_number() {
        let zf = parse_zone_file(concat!(
            "ok A 192.0.2.1\n",
            "a A 999.0.0.1\n",
            "b AAAA nope\n",
            "c MX ten mail.\n",
            "d SRV 1 2 mail.\n",
            "e CAA 0 issue\n",
            "f SOA ns. h. 1 2 3\n",
            "g TXT\n",
            "h CNAME\n",
            "i\n",
            "j 300 IN\n",
            "$TTL forever\n",
            "$ORIGIN\n",
            "k A 192.0.2.1 extra\n",
            "l TXT \"unterminated\n",
            "m \"A\" 192.0.2.1\n",
            "n A-B 1\n",
            "o 9TYPE x\n",
            "p SOA ns. h. 1 2 3 4 forever\n",
        ));
        assert_eq!(zf.records().count(), 1);
        let lines: Vec<usize> = zf.skipped.iter().map(|s| s.line).collect();
        assert_eq!(lines, (2..=19).collect::<Vec<_>>());
    }

    #[test]
    fn test_unbalanced_parenthesis_reports_the_record_start_line() {
        let zf = parse_zone_file("ok A 192.0.2.1\n@ SOA ns. h. ( 1 2\n3 4 5\n");
        assert_eq!(zf.records().count(), 1);
        assert_eq!(zf.skipped[0].line, 2);
    }

    #[test]
    fn test_ttl_values() {
        for (token, secs) in [
            ("0", Some(0)),
            ("90", Some(90)),
            ("1W2D", Some(777_600)),
            ("1h30m", Some(5400)),
            ("5s", Some(5)),
            ("h", None),
            ("1x", None),
            ("", None),
            ("99999999999", None),
        ] {
            assert_eq!(parse_ttl(token), secs, "{token}");
        }
    }

    #[test]
    fn test_name_helpers() {
        assert!(Name::new("@").is_at());
        assert!(!Name::new("www").is_at());
        assert_eq!(Name::new("www").as_str(), "www");
    }

    #[test]
    fn test_parse_zone_file_from_path() {
        let dir = crate::test_support::TempDir::new();
        let path = dir.write("db", "www A 192.0.2.1\n");
        assert_eq!(
            parse_zone_file_from_path(&path).unwrap().records().count(),
            1
        );
        let err = parse_zone_file_from_path(&dir.path().join("missing"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing"), "{err}");
    }
}
