//! Command-line interface.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
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
    /// When `--port` is set, also starts a lightweight HTTP server that
    /// serves the output directory — no external dependency needed.
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
        /// HTTP port to serve on. 0 means "serve is off".
        #[arg(long, default_value = "0")]
        port: u16,
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
            port,
        } => {
            info!("lagrange dev — build + watch ({interval}s poll)");
            let opts = BuildOptions {
                src: src.clone(),
                out: out.clone(),
                site_url: site_url.clone(),
            };
            site::build(&opts)?;

            // Start HTTP server if requested.
            let _server = if port > 0 {
                let out_d = out.clone();
                let bind = format!("127.0.0.1:{port}");
                info!("serving {} on http://{bind}", out_d.display());
                Some(serve_dir(out_d, port))
            } else {
                None
            };

            info!(
                "watching {} …  open {base}index.html or http://127.0.0.1:{port}/{default_lang}",
                src.display(),
                base = out.display().to_string() + "/",
                port = port,
                default_lang = if out.join("en").exists() { "en/" } else { "" },
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

// ── built-in HTTP static server (no deps — pure std) ─────────────────────

/// Spawn a daemon thread that serves `root` over HTTP on `port`. Returns the
/// handle so the caller can join or detach.
fn serve_dir(root: PathBuf, port: u16) -> thread::JoinHandle<()> {
    let root = Arc::new(root);
    let bind = format!("127.0.0.1:{port}");
    thread::spawn(move || {
        let listener = match std::net::TcpListener::bind(&bind) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("cannot bind {}: {e}", bind);
                return;
            }
        };
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let root = Arc::clone(&root);
            thread::spawn(move || handle_http(stream, &root));
        }
    })
}

fn handle_http(mut stream: std::net::TcpStream, root: &PathBuf) {
    use std::io::{BufRead, BufReader, Write};
    let reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| unreachable!()));
    let Some(req_line) = reader.lines().next().and_then(|l| l.ok()) else {
        return;
    };
    // Parse `GET /path HTTP/1.x`
    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 || !parts[0].eq_ignore_ascii_case("GET") {
        respond(&mut stream, 405, "text/plain", "Method Not Allowed");
        return;
    }
    let req_path = parts[1].trim_start_matches('/');
    let abs = if req_path.is_empty() {
        root.join("index.html")
    } else {
        root.join(req_path)
    };
    // Prevent directory traversal.
    if !abs.starts_with(root) {
        respond(&mut stream, 403, "text/plain", "Forbidden");
        return;
    }
    match std::fs::read(&abs) {
        Ok(bytes) => {
            let mime = mime_for(abs.extension().and_then(|e| e.to_str()).unwrap_or(""));
            let header = format!(
                "HTTP/1.0 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\r\n",
                bytes.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&bytes);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            respond(&mut stream, 404, "text/plain", "Not Found");
        }
        Err(_) => {
            respond(&mut stream, 500, "text/plain", "Internal Server Error");
        }
    }
}

fn respond(stream: &mut std::net::TcpStream, code: u16, mime: &str, body: &str) {
    use std::io::Write;
    let label = match code {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.0 {code} {label}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
}

/// Simple extension → MIME mapping (enough for a docs site).
fn mime_for(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" | "otf" => "font/opentype",
        "xml" => "application/xml",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
