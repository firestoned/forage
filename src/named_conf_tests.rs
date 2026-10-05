// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;

    fn zones(conf: &NamedConf) -> Vec<&ZoneStmt> {
        conf.statements
            .iter()
            .filter_map(|s| {
                if let Statement::Zone(z) = s {
                    Some(z)
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn test_zone_type_file_and_class() {
        let conf = parse_named_conf(
            "zone \"example.com\" IN { type master; file \"db.example.com\"; allow-transfer { none; }; };",
        )
        .unwrap();
        let z = zones(&conf)[0];
        assert_eq!(z.name, "example.com");
        assert_eq!(z.options.zone_type.as_deref(), Some("primary"));
        assert_eq!(z.options.file.as_deref(), Some("db.example.com"));
    }

    #[test]
    fn test_zone_types_are_normalized() {
        for (word, normalized) in [
            ("primary", "primary"),
            ("master", "primary"),
            ("secondary", "secondary"),
            ("slave", "secondary"),
            ("stub", "stub"),
            ("mirror", "mirror"),
        ] {
            let conf = parse_named_conf(&format!("zone \"z\" {{ type {word}; }};")).unwrap();
            assert_eq!(
                zones(&conf)[0].options.zone_type.as_deref(),
                Some(normalized),
                "{word}"
            );
        }
    }

    #[test]
    fn test_zone_without_block_or_type() {
        let conf = parse_named_conf("zone \"bare\";\nzone \"empty\" { };").unwrap();
        let zs = zones(&conf);
        assert_eq!(zs.len(), 2);
        assert_eq!(zs[0].options.zone_type, None);
        assert_eq!(zs[1].options.file, None);
    }

    #[test]
    fn test_options_include_view_and_other_statements() {
        let conf = parse_named_conf(
            r#"
            # hash comment
            // line comment
            /* block
               comment */
            options { directory "/var/named"; recursion no; };
            options { recursion yes; };
            include "/etc/bind/zones.conf";
            view "internal" { zone "inner" { type primary; }; };
            acl trusted { 192.0.2.0/24; };
            key "k" { algorithm hmac-sha256; secret "c2VjcmV0"; };
            "#,
        )
        .unwrap();
        assert_eq!(
            conf.statements,
            vec![
                Statement::Options(OptionsStmt {
                    directory: Some("/var/named".into())
                }),
                Statement::Options(OptionsStmt { directory: None }),
                Statement::Include("/etc/bind/zones.conf".into()),
                Statement::View("internal".into()),
                Statement::Other("acl".into()),
                Statement::Other("key".into()),
            ]
        );
    }

    #[test]
    fn test_quoted_strings_keep_braces_and_semicolons() {
        let conf = parse_named_conf("zone \"a;b{c}\" { file \"x // not a comment\"; };").unwrap();
        let z = zones(&conf)[0];
        assert_eq!(z.name, "a;b{c}");
        assert_eq!(z.options.file.as_deref(), Some("x // not a comment"));
    }

    #[test]
    fn test_syntax_errors_name_the_line() {
        for (input, needle) in [
            ("zone \"a\" {\n type primary;\n", "line 1: unclosed '{'"),
            ("};", "line 1: unexpected '}'"),
            ("zone \"a\"", "line 1: missing ';'"),
            ("zone \"a\n", "line 1: unterminated string"),
            ("/* never closed", "line 1: unterminated comment"),
            ("\n\n{ };", "line 3: block without a statement keyword"),
            ("zone {};", "line 1: zone without a name"),
        ] {
            let err = parse_named_conf(input).unwrap_err().to_string();
            assert!(err.contains(needle), "{input:?}: {err}");
        }
    }

    #[test]
    fn test_parse_named_conf_file_reads_and_reports_path() {
        let dir = crate::test_support::TempDir::new();
        let path = dir.write("named.conf", "zone \"f\" { type hint; };");
        assert_eq!(zones(&parse_named_conf_file(&path).unwrap())[0].name, "f");

        let broken = dir.write("broken.conf", "zone \"b\" {\n");
        let err = parse_named_conf_file(&broken).unwrap_err().to_string();
        assert!(err.contains("broken.conf: line 1: unclosed '{'"), "{err}");

        let missing = dir.path().join("nope.conf");
        let err = parse_named_conf_file(&missing).unwrap_err().to_string();
        assert!(err.contains("nope.conf"), "{err}");
    }
}
