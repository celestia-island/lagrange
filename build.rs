//! Build script: embed the browser-side comment runtime into the binary.
//!
//! The lagrange SSG ships a hand-written, framework-free Web Component
//! (`assets/lagrange-comments.js` + `.css`) that the comment mount points
//! reference. Previously `site.rs` looked that directory up on disk at run
//! time via `CARGO_MANIFEST_DIR` / exe-sibling heuristics — which breaks for a
//! prebuilt binary installed from crates.io (no source tree next to it).
//!
//! Following the hikari-components convention, this script copies the asset
//! files into `OUT_DIR` at compile time so the binary can `include_str!` them
//! and emit them into every build output with zero runtime filesystem lookup.
//!
//! Files are matched by extension so adding a future asset (e.g. a second
//! component) only requires dropping it in `assets/`.

use std::{fs, path::PathBuf};

const ASSET_EXTS: &[&str] = &["js", "css", "mjs", "svg"];

fn main() {
    // Declare the custom cfg so `#[cfg(lagrange_assets_empty)]` doesn't trip
    // the unexpected_cfgs lint.
    println!("cargo::rustc-check-cfg=cfg(lagrange_assets_empty)");

    // Re-run if any asset changes (cargo only hashes .rs by default).
    let assets_dir = PathBuf::from("assets");
    println!("cargo:rerun-if-changed=assets");

    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let dest = out_dir.join("lagrange_assets");
    fs::create_dir_all(&dest).expect("create OUT_DIR/lagrange_assets");

    let mut copied = 0usize;
    if assets_dir.is_dir() {
        for entry in fs::read_dir(&assets_dir).expect("read assets dir") {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let is_asset = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ASSET_EXTS.contains(&ext))
                .unwrap_or(false);
            if !is_asset {
                continue;
            }
            let name = entry.file_name();
            fs::copy(&path, dest.join(&name)).expect("copy asset to OUT_DIR");
            copied += 1;
        }
    }

    if copied == 0 {
        // No assets present (e.g. a stripped source tree). The site builder
        // handles a missing runtime gracefully (warns), so this is not fatal —
        // but emit a cfg so Rust code can distinguish "embedded empty" from
        // "embedded present" without a runtime probe.
        println!("cargo:rustc-cfg=lagrange_assets_empty");
    }

    println!(
        "cargo:warning=lagrange: embedded {copied} browser asset(s) into OUT_DIR"
    );
}
