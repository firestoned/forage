// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! forage: import an existing BIND9 named.conf into a bindy-managed
//! Kubernetes cluster as native CRD resources.
//!
//! ```text
//! forage --conf /etc/bind/named.conf | kubectl apply -f -
//! ```
//!
//! No third-party crates (ADR-0006): parsing, serialization, argument
//! handling and logging are modules of this crate.

#[macro_use]
mod log;
mod cli;
mod crd;
mod error;
mod json;
mod mapper;
mod named_conf;
#[cfg(test)]
mod test_support;
mod yaml;
mod zone_file;

use std::process::ExitCode;

use cli::{Cli, Command, OutputFormat};
use error::{Context, Result};
use json::Value;
use mapper::{Mapper, MapperConfig};

/// Exit status for usage errors, as clap used.
const USAGE_EXIT: u8 = 2;

fn main() -> ExitCode {
    match cli::parse(std::env::args().skip(1)) {
        Ok(Command::Help) => {
            print!("{}", cli::help_text());
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("{}", cli::version_text());
            ExitCode::SUCCESS
        }
        Ok(Command::Run(cli)) => {
            log::set_level(log::level_from(
                cli.debug,
                std::env::var("RUST_LOG").ok().as_deref(),
            ));
            match run(&cli) {
                Ok(output) => {
                    print!("{output}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("Error: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(usage) => {
            eprint!("{usage}");
            ExitCode::from(USAGE_EXIT)
        }
    }
}

/// Reads `cli.conf`, maps it, and renders the manifest stream.
///
/// # Errors
/// When named.conf cannot be read or parsed.
fn run(cli: &Cli) -> Result<String> {
    info!("forage v{} starting", env!("CARGO_PKG_VERSION"));
    info!("reading named.conf from {}", cli.conf.display());

    let named_conf = named_conf::parse_named_conf_file(&cli.conf)
        .with_context(|| format!("failed to parse {}", cli.conf.display()))?;

    let manifests = Mapper::new(MapperConfig {
        conf_path: cli.conf.clone(),
        zone_dir: cli.zone_dir.clone(),
        namespace: cli.namespace.clone(),
        cluster_ref: cli.cluster_ref.clone(),
        zone_filter: cli.zone_filter.clone(),
        skip_records: cli.skip_records,
        record_types: cli.record_types.clone(),
    })
    .map(&named_conf);

    info!("emitted {} manifest(s)", manifests.len());
    Ok(render(&manifests, cli.output))
}

/// Renders manifests as a YAML stream (`---` before each document) or as
/// concatenated pretty JSON objects, one per line group.
fn render(manifests: &[Value], format: OutputFormat) -> String {
    manifests
        .iter()
        .map(|m| match format {
            OutputFormat::Json => format!("{}\n", m.to_json_pretty()),
            OutputFormat::Yaml => format!("---\n{}", yaml::to_yaml(m)),
        })
        .collect()
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
