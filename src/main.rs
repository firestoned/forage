// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! forage — Import an existing BIND9 named.conf into a bindy-managed
//! Kubernetes cluster as native CRD resources.
//!
//! Phase 1: one-time YAML dump to stdout.
//!
//! ```text
//! forage --conf /etc/bind/named.conf | kubectl apply -f -
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

mod crd;
mod mapper;

use mapper::Mapper;

/// forage — convert a BIND9 named.conf into bindy Kubernetes CRD manifests
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to named.conf
    #[arg(long, short = 'c', default_value = "/etc/bind/named.conf")]
    conf: std::path::PathBuf,

    /// Base directory for resolving relative zone-file paths.
    /// Defaults to the `directory` option in named.conf, then the named.conf parent dir.
    #[arg(long)]
    zone_dir: Option<std::path::PathBuf>,

    /// Kubernetes namespace for emitted resources
    #[arg(long, default_value = "bindy-system")]
    namespace: String,

    /// Value for DNSZoneSpec.cluster_ref (optional)
    #[arg(long)]
    cluster_ref: Option<String>,

    /// Only export zones matching this glob pattern (e.g. "*.example.com")
    #[arg(long, default_value = "*")]
    zone_filter: String,

    /// Emit only DNSZone resources — skip individual record CRs
    #[arg(long)]
    skip_records: bool,

    /// Comma-separated record types to include (default: all).
    /// Valid values: A,AAAA,CNAME,MX,TXT,SRV,CAA
    #[arg(long)]
    record_types: Option<String>,

    /// Output format
    #[arg(long, default_value = "yaml", value_parser = ["yaml", "json"])]
    output: String,

    /// Enable debug-level logging
    #[arg(long, short = 'd')]
    debug: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.debug);

    info!("forage v{} starting", env!("CARGO_PKG_VERSION"));
    info!("reading named.conf from {}", cli.conf.display());

    let named_conf = hornet_bind9::parse_named_conf_file(&cli.conf)
        .with_context(|| format!("failed to parse {}", cli.conf.display()))?;

    let mapper = Mapper::new(mapper::MapperConfig {
        conf_path: cli.conf.clone(),
        zone_dir: cli.zone_dir,
        namespace: cli.namespace,
        cluster_ref: cli.cluster_ref,
        zone_filter: cli.zone_filter,
        skip_records: cli.skip_records,
        record_types: parse_record_types(cli.record_types.as_deref()),
    });

    let manifests = mapper.map(&named_conf)?;

    for manifest in &manifests {
        match cli.output.as_str() {
            "json" => println!("{}", serde_json::to_string_pretty(manifest)?),
            _ => {
                print!("---\n{}", serde_yaml::to_string(manifest)?);
            }
        }
    }

    info!("emitted {} manifest(s)", manifests.len());
    Ok(())
}

fn parse_record_types(input: Option<&str>) -> Option<Vec<String>> {
    input.map(|s| s.split(',').map(|t| t.trim().to_uppercase()).collect())
}

fn init_tracing(debug: bool) {
    let filter = if debug {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
