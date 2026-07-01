//! Command-line interface.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::site::{self, BuildOptions};

#[derive(Parser)]
#[command(
    name = "lagrange",
    version,
    about = "Lagrange — a pest-based markdown documentation renderer"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Build a documentation tree into a static HTML site.
    Build {
        /// Source docs root (one subdirectory per language). Defaults to `docs`.
        #[arg(long, default_value = "docs")]
        src: PathBuf,
        /// Output directory. Defaults to `target/site`.
        #[arg(long, default_value = "target/site")]
        out: PathBuf,
        /// Optional absolute site URL (e.g. https://lagrange.docs.celestia.world).
        #[arg(long)]
        site_url: Option<String>,
    },
}

/// Run the CLI.
pub fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Build { src, out, site_url } => {
            let opts = BuildOptions { src, out, site_url };
            site::build(&opts)?;
            Ok(())
        }
    }
}
