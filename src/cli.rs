//! Command-line interface.

use std::{path::PathBuf, time::Duration};

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
        /// Source docs root. Defaults to `docs`.
        #[arg(long, default_value = "docs")]
        src: PathBuf,
        /// Output directory. Defaults to `dist` (matching mdBook / Zola).
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Optional site URL.
        #[arg(long)]
        site_url: Option<String>,
        /// Default language. Defaults to "en".
        #[arg(long, default_value = "en")]
        default_lang: String,
    },
    /// Build once, then watch for changes and rebuild automatically.
    /// When `--port` is set, also starts a lightweight HTTP server that
    /// serves the output directory — no external dependency needed.
    Dev {
        /// Source docs root. Defaults to `docs`.
        #[arg(long, default_value = "docs")]
        src: PathBuf,
        /// Output directory. Defaults to `dist`.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Optional site URL.
        #[arg(long)]
        site_url: Option<String>,
        /// Default language (default "en"). Used when no query param, no
        /// localStorage, and no browser-preference match.
        #[arg(long, default_value = "en")]
        default_lang: String,
        /// Polling interval in seconds (default 1).
        #[arg(long, default_value = "1")]
        interval: f64,
        /// HTTP port to serve on. 0 picks a random available port on all
        /// interfaces and prints the chosen address.
        #[arg(long, default_value = "0")]
        port: u16,
    },
}

/// Run the CLI.
pub fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Build {
            src,
            out,
            site_url,
            default_lang,
        } => {
            let opts = BuildOptions {
                src,
                out,
                site_url,
                default_lang: Some(default_lang),
            };
            site::build(&opts)
        }
        Command::Dev {
            src,
            out,
            site_url,
            interval,
            port,
            default_lang,
        } => {
            info!("lagrange dev — build + watch ({interval}s poll)");
            let opts = BuildOptions {
                src: src.clone(),
                out: out.clone(),
                site_url: site_url.clone(),
                default_lang: Some(default_lang.clone()),
            };
            site::build(&opts)?;

            // Spawn the axum + tower-http static-file server on a tokio
            // runtime.  The runtime is kept alive for the lifetime of this
            // scope (i.e. until watch_loop returns).
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;

            let bind = if port > 0 {
                format!("0.0.0.0:{port}")
            } else {
                "0.0.0.0:0".to_string()
            };

            let bind_addr = rt.block_on(async {
                match tokio::net::TcpListener::bind(&bind).await {
                    Ok(listener) => {
                        let addr = listener.local_addr().unwrap().to_string();
                        info!("serving {} on http://{addr}", out.display());
                        let app = axum::Router::new()
                            .fallback_service(tower_http::services::ServeDir::new(out.clone()));
                        tokio::spawn(async move {
                            if let Err(e) = axum::serve(listener, app).await {
                                tracing::error!("HTTP server error: {e}");
                            }
                        });
                        addr
                    }
                    Err(e) => {
                        tracing::error!("cannot bind {bind}: {e}");
                        bind
                    }
                }
            });

            info!(
                "watching {} …  open http://{bind_addr}/index.html?lang={dl}",
                src.display(),
                dl = default_lang,
            );
            // watch_loop blocks forever; rt stays alive in this scope.
            watch_loop(src, out, site_url, default_lang, interval)?;
            Ok(())
        }
    }
}

/// Poll every `interval` seconds; when any file under `src` changes, rebuild.
fn watch_loop(
    src: PathBuf,
    out: PathBuf,
    site_url: Option<String>,
    default_lang: String,
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
                default_lang: Some(default_lang.clone()),
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
