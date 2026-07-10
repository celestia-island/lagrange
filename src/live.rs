//! Build-time live component compiler.
//!
//! Scans parsed markdown blocks for [`Block::LiveComponent`] entries, generates
//! a temporary Rust crate for each unique source snippet, compiles it against
//! `hikari-components` + `tairitsu-macros`, executes the binary to capture
//! rendered HTML, and returns a map of `source → html`.
//!
//! The generated crate wraps the user's `rsx!{...}` expression in a `main()`
//! that calls `VNode::render_to_html()` and prints the result. This is a
//! **build-time** operation — no runtime eval, no proc-macro-at-runtime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::{info, warn};

use crate::markdown::Block;

/// Collect all unique live component sources from a list of parsed blocks.
pub fn collect_sources(blocks: &[Block]) -> Vec<String> {
    let mut sources = Vec::new();
    collect_sources_inner(blocks, &mut sources);
    sources.sort();
    sources.dedup();
    sources
}

fn collect_sources_inner(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::LiveComponent { source } => out.push(source.clone()),
            Block::Blockquote(inner) => collect_sources_inner(inner, out),
            Block::Center(inner) => collect_sources_inner(inner, out),
            _ => {}
        }
    }
}

/// Compile and execute all live component snippets, returning a map of
/// `source → rendered_html`.
///
/// If compilation fails for a snippet, it is simply omitted from the map
/// (the renderer falls back to source-only display). Errors are logged but
/// do not fail the build.
pub fn compile_all(sources: &[String], work_dir: &Path) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if sources.is_empty() {
        return result;
    }

    let live_dir = work_dir.join("live-blocks");
    std::fs::create_dir_all(&live_dir).ok();

    for source in sources {
        match compile_one(source, &live_dir) {
            Ok(html) => {
                result.insert(source.clone(), html);
            }
            Err(e) => {
                warn!("live block compile failed: {e}");
            }
        }
    }

    info!(
        "compiled {} of {} live component block(s)",
        result.len(),
        sources.len()
    );
    result
}

/// Generate, compile, and execute a single live component snippet.
fn compile_one(source: &str, base_dir: &Path) -> anyhow::Result<String> {
    let hash = short_hash(source);
    let crate_dir = base_dir.join(&hash);

    // Generate the temporary crate.
    std::fs::create_dir_all(crate_dir.join("src"))?;
    std::fs::write(crate_dir.join("Cargo.toml"), render_cargo_toml(&hash))?;
    std::fs::write(
        crate_dir.join("src").join("main.rs"),
        render_main_rs(source),
    )?;

    // All live-block crates share a single target directory so the heavy
    // dependencies (tairitsu-vdom, hikari-components, etc.) are compiled
    // once and cached across all 600+ snippets. Without this, each crate
    // gets its own target/ and re-compiles the entire dep tree every time.
    let shared_target = base_dir.join("target");

    // Compile. Disable sccache / rustc-wrapper for these tiny single-file
    // crates — sccache adds overhead and can fail on certain rustc invocations
    // (e.g. "Compiler not supported" on Windows stable). The crates are too
    // small to benefit from caching anyway.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let build = Command::new(&cargo)
        .args(["build", "--release", "--quiet"])
        .env("RUSTC_WRAPPER", "")
        .env("CARGO_BUILD_RUSTC_WRAPPER", "")
        .env("CARGO_TARGET_DIR", &shared_target)
        .current_dir(&crate_dir)
        .output()?;

    if !build.status.success() {
        let stderr = String::from_utf8_lossy(&build.stderr);
        anyhow::bail!(
            "cargo build failed for live block {hash}: {}",
            stderr_tail(&stderr, 500)
        );
    }

    // Locate the compiled binary in the shared target dir.
    let bin = find_binary(&shared_target, &hash)
        .ok_or_else(|| anyhow::anyhow!("compiled binary not found for {hash}"))?;

    // Execute and capture stdout (the rendered HTML).
    let run = Command::new(&bin).output()?;
    if !run.status.success() {
        let stderr = String::from_utf8_lossy(&run.stderr);
        anyhow::bail!(
            "execution failed for live block {hash}: {}",
            stderr_tail(&stderr, 500)
        );
    }

    let html = String::from_utf8_lossy(&run.stdout).to_string();
    Ok(html.trim().to_string())
}

/// Generate the Cargo.toml for a temporary live-block crate.
///
/// Uses path dependencies to the local workspace so the crate compiles
/// against the same hikari-components/tairitsu versions lagrange uses.
/// The paths are resolved relative to the lagrange crate's CARGO_MANIFEST_DIR.
fn render_cargo_toml(hash: &str) -> String {
    // Resolve absolute paths to the dependency source directories.
    // Use forward slashes — TOML string literals choke on backslashes.
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let tairitsu_root = format!("{manifest}/../tairitsu/packages");
    let hikari_root = format!("{manifest}/../hikari/packages");
    format!(
        r#"[package]
name = "live-block-{hash}"
version = "0.0.0"
edition = "2021"
publish = false

[[bin]]
name = "live-block-{hash}"
path = "src/main.rs"

# Standalone workspace so cargo doesn't try to merge this into lagrange's
# workspace (the temp crate lives under lagrange's output dir).
[workspace]

[dependencies]
tairitsu-vdom = {{ path = "{tairitsu_root}/vdom" }}
tairitsu-hooks = {{ path = "{tairitsu_root}/hooks" }}
tairitsu-macros = {{ path = "{tairitsu_root}/macros" }}
hikari-components = {{ path = "{hikari_root}/components" }}
"#
    )
}

/// Generate the main.rs that wraps the user's rsx! expression.
///
/// The user's source is expected to be an `rsx!{...}` expression (or a
/// component call like `Button(ButtonProps {{ ... }})`). We wrap it in a
/// function that renders the resulting VNode to HTML.
fn render_main_rs(source: &str) -> String {
    // If the source already starts with `rsx!`, use it directly; otherwise
    // wrap it in rsx! so bare component calls work too.
    let trimmed = source.trim();
    let expr = if trimmed.starts_with("rsx")
        || trimmed.starts_with("Button")
        || trimmed.starts_with("Card")
    {
        trimmed.to_string()
    } else {
        format!("rsx! {{ {trimmed} }}")
    };

    format!(
        r#"use tairitsu_vdom::VNode;
use hikari_components::prelude::*;

fn main() {{
    let vnode: VNode = {expr};
    let html = vnode.render_to_html();
    print!("{{}}", html);
}}
"#
    )
}

fn short_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("lb{:016x}", h.finish())
}

fn find_binary(target_dir: &Path, name: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("live-block-{name}.exe")
    } else {
        format!("live-block-{name}")
    };
    // Search common target subdirs.
    for sub in &["release", "debug"] {
        let path = target_dir.join(sub).join(&exe_name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn stderr_tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max..])
    }
}
