//! Build-time live component compiler.
//!
//! Scans parsed markdown blocks for [`Block::LiveComponent`] entries, generates
//! a **single** temporary Rust workspace crate containing one binary per unique
//! source snippet, compiles them all in one `cargo build` invocation (so the
//! heavy dependencies — tairitsu-vdom, hikari-components — are compiled once
//! and shared), executes each binary to capture rendered HTML, and returns a
//! map of `source → html`.
//!
//! The generated workspace has one `[[bin]]` per snippet. This is dramatically
//! faster than per-snippet crates because the dependency tree is resolved and
//! compiled exactly once.

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
    // Clean any stale state from a previous run so we don't hit lock files
    // or stale fingerprints.
    if live_dir.exists() {
        std::fs::remove_dir_all(&live_dir).ok();
    }
    std::fs::create_dir_all(&live_dir).ok();

    // Generate a single workspace crate with one [[bin]] per snippet.
    let crate_dir = live_dir.join("workspace");
    std::fs::create_dir_all(crate_dir.join("src")).ok();

    // Map each source to its hash + binary name for later lookup.
    let entries: Vec<(String, String)> = sources
        .iter()
        .map(|s| {
            let hash = short_hash(s);
            (s.clone(), hash)
        })
        .collect();

    // Write each snippet as src/<hash>.rs
    for (source, hash) in &entries {
        let file_path = crate_dir.join("src").join(format!("{hash}.rs"));
        std::fs::write(&file_path, render_bin_rs(source)).ok();
    }

    // Generate Cargo.toml with all [[bin]] entries.
    std::fs::write(crate_dir.join("Cargo.toml"), render_cargo_toml(&entries)).ok();

    // Compile all binaries in one invocation.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let build = Command::new(&cargo)
        .args(["build", "--release", "--quiet"])
        .env("RUSTC_WRAPPER", "")
        .env("CARGO_BUILD_RUSTC_WRAPPER", "")
        .current_dir(&crate_dir)
        .output();

    let build = match build {
        Ok(o) => o,
        Err(e) => {
            warn!("live block cargo invocation failed: {e}");
            return result;
        }
    };

    if !build.status.success() {
        let stderr = String::from_utf8_lossy(&build.stderr);
        // The error message may mention specific crate names, but we log
        // the tail for diagnostics and mark ALL snippets as failed.
        warn!(
            "live block workspace build failed ({} snippets): {}",
            entries.len(),
            stderr_tail(&stderr, 800)
        );
        return result;
    }

    // Execute each compiled binary.
    let target_dir = crate_dir.join("target");
    for (source, hash) in &entries {
        let bin = find_binary(&target_dir, hash);
        match bin {
            Some(path) => {
                let run = Command::new(&path).output();
                match run {
                    Ok(r) if r.status.success() => {
                        let html = String::from_utf8_lossy(&r.stdout).to_string();
                        result.insert(source.clone(), html.trim().to_string());
                    }
                    Ok(r) => {
                        let stderr = String::from_utf8_lossy(&r.stderr);
                        warn!(
                            "live block execution failed for {hash}: {}",
                            stderr_tail(&stderr, 300)
                        );
                    }
                    Err(e) => {
                        warn!("live block execution error for {hash}: {e}");
                    }
                }
            }
            None => {
                // Binary wasn't built — likely a compile error in this
                // specific snippet. The workspace build already logged it.
                warn!("live block binary not found for {hash}");
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

/// Generate the Cargo.toml for the workspace crate with all [[bin]] entries.
fn render_cargo_toml(entries: &[(String, String)]) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let tairitsu_root = format!("{manifest}/../tairitsu/packages");
    let hikari_root = format!("{manifest}/../hikari/packages");

    let mut bins = String::new();
    for (_, hash) in entries {
        bins.push_str(&format!(
            "[[bin]]\nname = \"live-block-{hash}\"\npath = \"src/{hash}.rs\"\n\n"
        ));
    }

    format!(
        r#"[package]
name = "live-block-workspace"
version = "0.0.0"
edition = "2021"
publish = false

# Standalone workspace so cargo doesn't try to merge this into lagrange's
# workspace (the temp crate lives under lagrange's output dir).
[workspace]

{bins}[dependencies]
tairitsu-vdom = {{ path = "{tairitsu_root}/vdom" }}
tairitsu-hooks = {{ path = "{tairitsu_root}/hooks" }}
tairitsu-macros = {{ path = "{tairitsu_root}/macros" }}
hikari-components = {{ path = "{hikari_root}/components" }}
"#
    )
}

/// Generate a single binary source file for one snippet.
fn render_bin_rs(source: &str) -> String {
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

fn find_binary(target_dir: &Path, hash: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("live-block-{hash}.exe")
    } else {
        format!("live-block-{hash}")
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
