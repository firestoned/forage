// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Command-line parsing (ADR-0006: replaces `clap`). Messages and the exit
//! code for usage errors (2) follow clap's conventions.

use std::fmt;
use std::path::PathBuf;

const DEFAULT_CONF: &str = "/etc/bind/named.conf";
const DEFAULT_NAMESPACE: &str = "bindy-system";
const DEFAULT_ZONE_FILTER: &str = "*";
const USAGE_FOOTER: &str = "\nUsage: forage [OPTIONS]\n\nFor more information, try '--help'.\n";

/// Output format for the manifest stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Yaml,
    Json,
}

/// Parsed options for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub conf: PathBuf,
    pub zone_dir: Option<PathBuf>,
    pub namespace: String,
    pub cluster_ref: Option<String>,
    pub zone_filter: String,
    pub skip_records: bool,
    /// Upper-cased, trimmed record types, if filtered.
    pub record_types: Option<Vec<String>>,
    pub output: OutputFormat,
    pub debug: bool,
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run(Cli),
    Help,
    Version,
}

/// A usage error, displayed as clap would (message, usage, hint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}\n{USAGE_FOOTER}", self.0)
    }
}

/// (long name, value placeholder or "" for a switch)
const OPTIONS: [(&str, &str); 9] = [
    ("conf", "CONF"),
    ("zone-dir", "ZONE_DIR"),
    ("namespace", "NAMESPACE"),
    ("cluster-ref", "CLUSTER_REF"),
    ("zone-filter", "ZONE_FILTER"),
    ("skip-records", ""),
    ("record-types", "RECORD_TYPES"),
    ("output", "OUTPUT"),
    ("debug", ""),
];

/// Parses arguments (without the program name).
///
/// # Errors
/// Returns a [`UsageError`] for unknown, repeated or malformed arguments.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, UsageError> {
    let mut seen: Vec<(&str, Option<String>)> = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (name, inline_value) = match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" => ("conf", None),
            "-d" => ("debug", None),
            long if long.starts_with("--") => match long[2..].split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (&long[2..], None),
            },
            _ => return Err(UsageError(format!("unexpected argument '{arg}' found"))),
        };
        let Some(&(name, placeholder)) = OPTIONS.iter().find(|(n, _)| *n == name) else {
            return Err(UsageError(format!("unexpected argument '{arg}' found")));
        };
        let shown = if placeholder.is_empty() {
            format!("--{name}")
        } else {
            format!("--{name} <{placeholder}>")
        };
        if seen.iter().any(|(n, _)| *n == name) {
            return Err(UsageError(format!(
                "the argument '{shown}' cannot be used multiple times"
            )));
        }
        let value = match (placeholder.is_empty(), inline_value) {
            (true, Some(v)) => {
                return Err(UsageError(format!(
                    "unexpected value '{v}' for '{shown}' found"
                )))
            }
            (true, None) => None,
            (false, Some(v)) => Some(v),
            (false, None) => Some(args.next().ok_or_else(|| {
                UsageError(format!(
                    "a value is required for '{shown}' but none was supplied"
                ))
            })?),
        };
        seen.push((name, value));
    }
    build(&seen).map(Command::Run)
}

fn build(seen: &[(&str, Option<String>)]) -> Result<Cli, UsageError> {
    let value = |name: &str| {
        seen.iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| v.clone())
    };
    let flag = |name: &str| seen.iter().any(|(n, _)| *n == name);
    let output = match value("output").as_deref() {
        None | Some("yaml") => OutputFormat::Yaml,
        Some("json") => OutputFormat::Json,
        Some(other) => {
            return Err(UsageError(format!(
                "invalid value '{other}' for '--output <OUTPUT>'\n  [possible values: yaml, json]"
            )))
        }
    };
    Ok(Cli {
        conf: PathBuf::from(value("conf").unwrap_or_else(|| DEFAULT_CONF.into())),
        zone_dir: value("zone-dir").map(PathBuf::from),
        namespace: value("namespace").unwrap_or_else(|| DEFAULT_NAMESPACE.into()),
        cluster_ref: value("cluster-ref"),
        zone_filter: value("zone-filter").unwrap_or_else(|| DEFAULT_ZONE_FILTER.into()),
        skip_records: flag("skip-records"),
        record_types: value("record-types")
            .map(|list| list.split(',').map(|t| t.trim().to_uppercase()).collect()),
        output,
        debug: flag("debug"),
    })
}

/// `forage <version>`.
pub fn version_text() -> String {
    format!("forage {}", env!("CARGO_PKG_VERSION"))
}

/// The `--help` text.
pub fn help_text() -> String {
    format!(
        "{about}\n\nUsage: forage [OPTIONS]\n\nOptions:\n\
  -c, --conf <CONF>                  Path to named.conf [default: {DEFAULT_CONF}]\n\
      --zone-dir <ZONE_DIR>          Base directory for relative zone-file paths\n\
                                     (default: named.conf `directory`, then named.conf's directory)\n\
      --namespace <NAMESPACE>        Namespace for emitted resources [default: {DEFAULT_NAMESPACE}]\n\
      --cluster-ref <CLUSTER_REF>    Value for DNSZone spec.clusterRef\n\
      --zone-filter <ZONE_FILTER>    Only zones matching this glob, e.g. \"*.example.com\" [default: *]\n\
      --skip-records                 Emit only DNSZone resources\n\
      --record-types <RECORD_TYPES>  Comma-separated types to include (default: all):\n\
                                     A,AAAA,CNAME,MX,TXT,SRV,CAA\n\
      --output <OUTPUT>              Output format [default: yaml] [possible values: yaml, json]\n\
  -d, --debug                        Debug logging on stderr\n\
  -h, --help                         Print help\n\
  -V, --version                      Print version\n",
        about = env!("CARGO_PKG_DESCRIPTION")
    )
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
