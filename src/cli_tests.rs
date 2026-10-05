// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::PathBuf;

    fn run_args(args: &[&str]) -> Cli {
        match parse(args.iter().map(|s| (*s).to_string())) {
            Ok(Command::Run(cli)) => cli,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    fn usage_error(args: &[&str]) -> String {
        match parse(args.iter().map(|s| (*s).to_string())) {
            Err(e) => e.to_string(),
            other => panic!("expected an error, got {other:?}"),
        }
    }

    #[test]
    fn test_defaults() {
        let cli = run_args(&[]);
        assert_eq!(cli.conf, PathBuf::from("/etc/bind/named.conf"));
        assert_eq!(cli.zone_dir, None);
        assert_eq!(cli.namespace, "bindy-system");
        assert_eq!(cli.cluster_ref, None);
        assert_eq!(cli.zone_filter, "*");
        assert!(!cli.skip_records);
        assert_eq!(cli.record_types, None);
        assert_eq!(cli.output, OutputFormat::Yaml);
        assert!(!cli.debug);
    }

    #[test]
    fn test_every_flag_long_short_and_equals_forms() {
        let cli = run_args(&[
            "-c",
            "/tmp/n.conf",
            "--zone-dir=/z",
            "--namespace",
            "dns",
            "--cluster-ref",
            "prod",
            "--zone-filter",
            "*.com",
            "--skip-records",
            "--record-types",
            " a , Mx",
            "--output=json",
            "-d",
        ]);
        assert_eq!(cli.conf, PathBuf::from("/tmp/n.conf"));
        assert_eq!(cli.zone_dir, Some(PathBuf::from("/z")));
        assert_eq!(cli.namespace, "dns");
        assert_eq!(cli.cluster_ref.as_deref(), Some("prod"));
        assert_eq!(cli.zone_filter, "*.com");
        assert!(cli.skip_records);
        assert_eq!(
            cli.record_types,
            Some(vec!["A".to_string(), "MX".to_string()])
        );
        assert_eq!(cli.output, OutputFormat::Json);
        assert!(cli.debug);
        assert_eq!(
            run_args(&["--conf", "x", "--debug"]).conf,
            PathBuf::from("x")
        );
    }

    #[test]
    fn test_help_and_version() {
        for flag in ["-h", "--help"] {
            assert!(matches!(parse([flag.to_string()]), Ok(Command::Help)));
        }
        for flag in ["-V", "--version"] {
            assert!(matches!(parse([flag.to_string()]), Ok(Command::Version)));
        }
        assert_eq!(
            version_text(),
            format!("forage {}", env!("CARGO_PKG_VERSION"))
        );
        let help = help_text();
        for needle in [
            "Usage: forage [OPTIONS]",
            "--conf <CONF>",
            "--zone-dir",
            "--namespace",
            "--cluster-ref",
            "--zone-filter",
            "--skip-records",
            "--record-types",
            "--output",
            "--debug",
            "--help",
            "--version",
        ] {
            assert!(help.contains(needle), "help lacks {needle}");
        }
    }

    #[test]
    fn test_usage_errors() {
        assert!(usage_error(&["--output", "toml"])
            .contains("invalid value 'toml' for '--output <OUTPUT>'"));
        assert!(usage_error(&["--namespace", "a", "--namespace", "b"])
            .contains("'--namespace <NAMESPACE>' cannot be used multiple times"));
        assert!(usage_error(&["--debug", "--debug"])
            .contains("'--debug' cannot be used multiple times"));
        assert!(usage_error(&["--conf"]).contains("a value is required for '--conf <CONF>'"));
        assert!(usage_error(&["--bogus"]).contains("unexpected argument '--bogus'"));
        assert!(usage_error(&["--debug=yes"]).contains("unexpected value 'yes' for '--debug'"));
        assert!(usage_error(&["stray"]).contains("unexpected argument 'stray'"));
        assert!(usage_error(&["--bogus"]).ends_with("For more information, try '--help'.\n"));
    }
}
