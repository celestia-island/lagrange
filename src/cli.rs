//! Command-line interface.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

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
    /// Build once, then watch for changes and rebuild automatically.
    Dev {
        /// Source docs root. Defaults to `docs`.
        #[arg(long, default_value = "docs")]
        src: PathBuf,
        /// Output directory. Defaults to `target/site`.
        #[arg(long, default_value = "target/site")]
        out: PathBuf,
        /// Optional site URL.
        #[arg(long)]
        site_url: Option<String>,
        /// Polling interval in seconds (default 1).
        #[arg(long, default_value = "1")]
        interval: f64,
    },
}

/// Run the CLI.
pub fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Build { src, out, site_url } => {
            let opts = BuildOptions { src, out, site_url };
            site::build(&opts)
        }
        Command::Dev {
            src,
            out,
            site_url,
            interval,
        } => {
            info!("lagrange dev — build + watch ({interval}s poll)");
            let opts = BuildOptions {
                src: src.clone(),
                out: out.clone(),
                site_url: site_url.clone(),
            };
            site::build(&opts)?;
            info!(
                "watching {} for changes …  (open {}index.html or serve with e.g. `python3 -m http.server`)",
                src.display(),
                out.display().to_string() + "/",
            );
            watch_loop(src, out, site_url, interval)?;
            Ok(())
        }
    }
}

/// Poll every `interval` seconds; when any file under `src` changes, rebuild.
fn watch_loop(
    src: PathBuf,
    out: PathBuf,
    site_url: Option<String>,
    interval: f64,
) -> anyhow::Result<()> {
    let interval = Duration::from_secs_f64(interval.max(0.2));
    // Snapshot all files under `src` with their modification times.
    let mut last_mtimes = snapshot_mtimes(&src)?;
    loop {
        std::thread::sleep(interval);
        let current = match snapshot_mtimes(&src) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("cannot read {}: {e}", src.display());
                continue;
            }
        };
        if current != last_mtimes {
            info!("files changed; rebuilding …");
            let opts = BuildOptions {
                src: src.clone(),
                out: out.clone(),
                site_url: site_url.clone(),
            };
            if let Err(e) = site::build(&opts) {
                tracing::error!("rebuild failed: {e:?}");
            }
            last_mtimes = current;
        }
    }
}

/// Collect (relative_path, modified_time) for every file under `root`.
fn snapshot_mtimes(root: &PathBuf) -> anyhow::Result<Vec<(String, std::time::SystemTime)>> {
    let mut out = Vec::new();
    collect_mtimes(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_mtimes(
    root: &PathBuf,
    dir: &PathBuf,
    out: &mut Vec<(String, std::time::SystemTime)>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_mtimes(root, &path, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified())?;
            out.push((rel.to_string_lossy().to_string(), mtime));
        }
    }
    Ok(())
}
