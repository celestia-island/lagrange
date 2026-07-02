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

            // Start HTTP server if requested.
            let bind_addr = if port > 0 {
                let bind = format!("0.0.0.0:{port}");
                info!("serving {} on http://{bind}", out.display());
                let _ = serve_dir(out.clone(), &bind);
                bind
            } else {
                // Port 0: pick a random available port on all interfaces.
                let bind = "0.0.0.0:0";
                let actual = serve_dir(out.clone(), bind);
                info!("serving {} on http://{actual}", out.display());
                actual
            };

            info!(
                "watching {} …  open http://{bind_addr}/index.html?lang={dl}",
                src.display(),
                bind_addr = bind_addr,
                dl = default_lang,
            );
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

// ── built-in HTTP static server (no deps — pure std) ─────────────────────

/// Spawn a daemon thread that serves `root` over HTTP on `bind` (e.g.
/// `"0.0.0.0:0"` or `"0.0.0.0:8080"`). Returns the actual local address
/// string that is being listened on.
fn serve_dir(root: PathBuf, bind: &str) -> String {
    let root = Arc::new(root);
    let listener = match std::net::TcpListener::bind(bind) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("cannot bind {bind}: {e}");
            return bind.to_string();
        }
    };
    let addr = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| bind.to_string());
    let root2 = Arc::clone(&root);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let root = Arc::clone(&root2);
            thread::spawn(move || handle_http(stream, &root));
        }
    });
    addr
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
