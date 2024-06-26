//! # reqaz
//! 
//! Requests from A to Z (reqaz) is a tool to help manage varions aspects of static HTML pages. We use it to help bundle things like CSS and certain HTML assets ahead of time before deploying to a bucket.
//! 
//! This isn't quite ready to use, but it's almost ready for us to use. Once it is, we will provide instructions for others as well.

#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

// Current requirement, might fix later idk
#![allow(clippy::multiple_crate_versions)]

// Remove clippy contradictions here
#![allow(clippy::blanket_clippy_restriction_lints)]
#![allow(clippy::implicit_return)]
#![allow(clippy::unseparated_literal_suffix)]

use clap::{Parser, Subcommand};
use color_eyre::Result;
use core::net::SocketAddr;
use core::str::FromStr;
use eyre::eyre;
use http::uri::{Uri, Authority};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use reqaz::source::{SourceResolver, SourceService};
use serde::{Serialize, Deserialize};
use std::env::current_dir;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::task::spawn as tokio_spawn;


/// Requests from A to Z
#[derive(Parser)]
struct Cli {
    /// The path to serve from
    #[arg(
        short = 'C'
    )]
    path: Option<PathBuf>,

    /// The port to serve from
    #[arg(
        short = 'p',
        long = "port",
    )]
    port: Option<u16>,

    /// Whether to print logs on request status
    #[arg(
        long = "log"
    )]
    log: Option<bool>,

    /// Subcommand to run
    #[clap(subcommand)]
    subcommand: Option<SubCli>
}

/// Subcommand to run
#[derive(Subcommand)]
enum SubCli {
    /// Serve files in root, do not build pipelines
    Serve
}

#[tokio::main]
#[allow(clippy::question_mark_used)]
#[allow(clippy::absolute_paths)]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();
    let config = {
        let base_config = CliConfig::default();

        if PathBuf::from("./reqaz.json").exists() {
            let json_contents = tokio::fs::read_to_string("./reqaz.json").await?;
            let json_config: CliConfig = serde_json::from_str(&json_contents)?;

            Ok::<CliConfig, eyre::Error>(json_config.override_with_cli(&args))
        } else {
            Ok(base_config.override_with_cli(&args))
        }
    }?;

    let authority = Authority::from_str(
        &format!("localhost:{}", config.port)
    )?;

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await?;

    let root = config.clone().root.or_else(|| {
        current_dir().ok()
    }).ok_or(eyre!("No root path provided"))?;

    let generate_config = config.generate;
    let resolver = SourceResolver::new(root, authority);

    let generate_optional = {
        if matches!(args.subcommand, Some(SubCli::Serve)) {
            None
        } else {
            generate_config
        }
    };

    #[allow(clippy::print_stdout)]
    if let Some(generate) = generate_optional {
        for pipeline in &generate.pipelines {
            let out_path = generate.output_dir.clone().join(pipeline.output.clone());

            if let Ok(resolved) = resolver.resolve_source(&pipeline.input) {
                tokio::fs::create_dir_all(out_path.parent().unwrap_or(&generate.output_dir)).await?;
                tokio::fs::write(out_path, resolved.body).await?;
            }
        }

        println!("Generated {} pipelines.", generate.pipelines.len());

        Ok(())
    } else {
        let service = SourceService::new(
            resolver,
            config.log
        );

        // gee thanks clippy, that's the whole point
        #[allow(clippy::infinite_loop)]
        loop {
            let accepted = listener.accept().await;

            if let Ok((stream, _)) = accepted {
                let io = TokioIo::new(stream);

                let service_clone = service.clone();

                #[allow(clippy::print_stderr)]
                tokio_spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, &service_clone)
                        .await
                    {
                        eprintln!("Error serving request: {err}");
                    }
                });
            }
        }
    }
}

/// Base CLI configuration (reqaz.json)
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct CliConfig {
    /// The root folder to serve from
    pub root: Option<PathBuf>,

    /// The port to serve from
    pub port: u16,

    /// Enable logging
    pub log: bool,

    /// Generate options
    pub generate: Option<GenerateConfig>
}

impl CliConfig {
    /// Override config with CLI options manually
    pub fn override_with_cli(mut self, cli: &Cli) -> Self {
        if let Some(root) = cli.path.clone() {
            self.root = Some(root);
        }

        if let Some(port) = cli.port {
            self.port = port;
        }

        if let Some(log) = cli.log {
            self.log = log;
        }

        self
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            root: None,
            port: 5000,
            log: false,
            generate: None
        }
    }
}

/// Generation configuration
#[derive(Serialize, Deserialize, Clone)]
struct GenerateConfig {
    /// The final output directory of files
    pub output_dir: PathBuf,

    /// List of pipelines to run
    pub pipelines: Vec<PipelineConfig>
}

/// Pipeline configuration
#[derive(Serialize, Deserialize, Clone)]
struct PipelineConfig {
    /// The input URI
    #[serde(with = "http_serde::uri")]
    pub input: Uri,

    /// The output path, relative to the output dir
    pub output: PathBuf
}
