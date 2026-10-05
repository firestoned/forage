// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn cli_for(conf: PathBuf, output: OutputFormat) -> Cli {
        match cli::parse(["--namespace".to_string(), "forage-test".to_string()]) {
            Ok(Command::Run(cli)) => Cli {
                conf,
                output,
                ..cli
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn test_run_matches_golden_outputs() {
        for name in ["basic", "edge"] {
            for (format, ext) in [(OutputFormat::Yaml, "yaml"), (OutputFormat::Json, "json")] {
                let out = run(&cli_for(fixture(name).join("named.conf"), format)).unwrap();
                let golden =
                    std::fs::read_to_string(fixture(name).join(format!("expected.{ext}"))).unwrap();
                assert_eq!(out, golden, "{name} {ext}");
            }
        }
    }

    #[test]
    fn test_run_reports_unreadable_conf() {
        let err = run(&cli_for(
            PathBuf::from("/nonexistent/forage.conf"),
            OutputFormat::Yaml,
        ))
        .unwrap_err();
        assert!(
            err.to_string()
                .starts_with("failed to parse /nonexistent/forage.conf: "),
            "{err}"
        );
    }

    #[test]
    fn test_render_empty_stream() {
        assert_eq!(render(&[], OutputFormat::Yaml), "");
        assert_eq!(render(&[], OutputFormat::Json), "");
    }
}
