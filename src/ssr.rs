//! Optional server-side rendering integration.
//!
//! When Node.js is available and the webui is installed, the build calls
//! the webui's SSR script to pre-render the Vue app to HTML. The result
//! is injected into the static page templates, so the initial paint is
//! fully rendered (no client-side flash).
//!
//! When Node.js is unavailable, the build silently falls back to
//! client-only rendering (an empty `<div id="app">` placeholder).
//! This is the progressive-enhancement pattern: the static site works
//! without Node, but gets faster first paint with it.

use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

/// Check whether Node.js is available on the PATH.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check whether the webui SSR prerequisites are in place.
fn ssr_ready() -> bool {
    let webui = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/webui");
    let script = webui.join("scripts/ssr-render.mjs");
    let modules = webui.join("node_modules/.pnpm");
    script.exists() && modules.exists()
}

/// Attempt to pre-render the Vue app to an HTML string.
///
/// Returns `None` when Node.js or the webui is unavailable, or when the
/// SSR script fails. The caller falls back to client-only rendering.
pub fn try_render_ssr() -> Option<String> {
    if !node_available() {
        info!("SSR skipped: node not found on PATH");
        return None;
    }
    if !ssr_ready() {
        info!("SSR skipped: packages/webui not installed (run pnpm install)");
        return None;
    }

    let webui = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/webui");
    let script = webui.join("scripts/ssr-render.mjs");

    info!("SSR: running node script for pre-rendered HTML…");
    let output = Command::new("node")
        .arg(&script)
        .current_dir(&webui)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let html = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if html.is_empty() {
                warn!("SSR script produced empty output");
                None
            } else {
                Some(html)
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            warn!(
                "SSR script failed (exit {}): {}",
                o.status,
                stderr.chars().take(200).collect::<String>()
            );
            None
        }
        Err(e) => {
            warn!("SSR script spawn failed: {e}");
            None
        }
    }
}
