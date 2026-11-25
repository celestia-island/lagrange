//! The drift lock for the TypeScript sources Rust embeds at compile time.
//!
//! `src/comments/{scrollbar,modal,clipboard,ui,runtime}.ts` and `src/diagram.ts`
//! are the SOURCES; the `.js` beside each one is compiled output that
//! `include_str!` embeds into the binary. Committing emitted bytes is what makes
//! a bare `cargo build` (and the crates.io tarball) work, and it is exactly the
//! arrangement that rots: somebody edits a source and forgets to compile, or
//! somebody — the path of least resistance — edits the `.js` directly and the
//! next compile silently deletes it.
//!
//! Two failure modes, both red here:
//!
//! 1. **The pipeline disagrees with the committed artifact** — stale output, a
//!    hand edit, a source edited without a rebuild.
//! 2. **The set disagrees with the tree** — an asset declared by neither side,
//!    or a declared artifact that does not exist.
//!
//! Resolved the same way either way: change the source, then
//! `node tools/scripts/build-embedded-ts.ts` (or `just embedded`). The lock
//! compares against THAT pipeline — never against a snapshot — so it cannot
//! drift from the thing it claims to pin.
//!
//! Two halves, because each covers what the other cannot:
//!
//! * [`drift_check_reports_nothing`] is the lock itself. It needs Node, so on a
//!   machine without Node it **fails closed** rather than passing vacuously: an
//!   unverifiable artifact is not a verified one. Escape hatch for a
//!   deliberately Node-less environment: `LAGRANGE_ALLOW_NO_NODE=1`, recorded
//!   in the PR rather than silently assumed.
//! * [`the_embedded_manifest_matches_the_rust_include_sites`] is the Rust half
//!   and never needs Node: the manifest in `build-embedded-ts.ts` must equal
//!   the `include_str!` call sites in `src/`, one for one, in both directions.
//!   A `.ts` that compiles but is never embedded is dead weight; an
//!   `include_str!` on a file nobody compiles is the rot coming back.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest_path() -> PathBuf {
    repo_root().join("tools/scripts/build-embedded-ts.ts")
}

/// The `"<path>"` literals inside the TS manifest's `ASSETS` array.
///
/// Read from the source rather than by importing it: this test must work
/// without Node, and the array is the contract only because the compiler reads
/// the same literal. Scoping to the `ASSETS` block keeps a comment mention of a
/// path from counting as a declaration.
fn declared_assets() -> BTreeSet<String> {
    let text =
        std::fs::read_to_string(manifest_path()).expect("read tools/scripts/build-embedded-ts.ts");
    let start = text
        .find("export const ASSETS")
        .expect("build-embedded-ts.ts must export const ASSETS");
    // The array opens at the first `[` after the declaration — the annotation
    // (`readonly Asset[]`) contains `[]`, so search from the `=` rather than
    // for the first bracket in the region. The OTHER `];` search used to be
    // from the `=`, which silently cut the region short the first time an
    // artifact's own code contained `];` (a `class … {};` line): the declared
    // set came back empty and this test would have passed vacuously. Search
    // for `];` only AFTER the array's opening bracket.
    let eq = start
        + text[start..]
            .find('=')
            .expect("an `=` after the declaration");
    let open = eq + text[eq..].find('[').expect("an `[` after the declaration");
    let end = open
        + text[open..]
            .find("];")
            .expect("the ASSETS array must terminate with `];`");
    let mut paths = BTreeSet::new();
    for line in text[start..end].lines() {
        for field in ["source", "artifact"] {
            let marker = format!("{field}: \"");
            let Some(at) = line.find(&marker) else {
                continue;
            };
            let rest = &line[at + marker.len()..];
            let close = rest.find('"').expect("a closing quote for the path");
            paths.insert(rest[..close].to_string());
        }
    }
    paths
}

/// The `include_str!` target of every call site in `src/`, repo-relative.
fn embedded_in_rust() -> BTreeSet<String> {
    let src = repo_root().join("src");
    let mut found = BTreeSet::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).expect("read a src/ directory");
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read a source file");
            let parent = path.parent().expect("a source file has a parent");
            for (idx, _) in text.match_indices("include_str!(\"") {
                let rest = &text[idx + "include_str!(\"".len()..];
                let close = rest.find('"').expect("a closing quote");
                let target = parent.join(&rest[..close]);
                let rel = target
                    .strip_prefix(repo_root())
                    .expect("include_str! inside the repo");
                found.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn compile_script() -> PathBuf {
    repo_root().join("tools/scripts/build-embedded-ts.ts")
}

/// Runs the compiler in `--check` mode and returns its exit status + output.
fn check_pipeline() -> (bool, String) {
    let output = Command::new("node")
        .arg(compile_script())
        .arg("--check")
        .current_dir(repo_root())
        .output()
        .expect("spawn node tools/scripts/build-embedded-ts.ts --check");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text.trim().to_string())
}

#[test]
fn drift_check_reports_nothing() {
    if !node_available() {
        if std::env::var("LAGRANGE_ALLOW_NO_NODE").as_deref() == Ok("1") {
            eprintln!(
                "embedded_ts: SKIPPED — node is not on PATH and LAGRANGE_ALLOW_NO_NODE=1. \
                 The committed artifacts are NOT verified in this run."
            );
            return;
        }
        panic!(
            "embedded_ts: node is not on PATH, so the committed embedded artifacts cannot be \
             verified. Install Node (the compile needs it), or set LAGRANGE_ALLOW_NO_NODE=1 to \
             record that this run deliberately went unverified."
        );
    }
    let (ok, text) = check_pipeline();
    assert!(
        ok,
        "the committed embedded artifacts must equal what \
         `node tools/scripts/build-embedded-ts.ts` emits — rebuild with \
         `node tools/scripts/build-embedded-ts.ts` (or `just embedded`) and commit the .js.\n{text}"
    );
    // Guard against a compile that "passes" while covering nothing (the
    // zero-rows-read-as-green shape): the check must report on exactly the
    // declared assets. `declared_assets()` returns sources AND artifacts, so
    // the asset count is half of it.
    let assets = declared_assets().len() / 2;
    assert!(
        text.contains(&format!("{assets} artifacts match")),
        "the compile must report on every declared asset; declared={assets} output:\n{text}"
    );
}

#[test]
fn the_embedded_manifest_matches_the_rust_include_sites() {
    let declared = declared_assets();
    assert!(
        !declared.is_empty(),
        "ASSETS must declare the embedded sources"
    );
    let embedded = embedded_in_rust();

    // Every artifact the compiler emits must be the file Rust actually embeds.
    let declared_artifacts: BTreeSet<String> = declared
        .iter()
        .filter(|p| p.ends_with(".js"))
        .cloned()
        .collect();
    let declared_sources: BTreeSet<String> = declared
        .iter()
        .filter(|p| p.ends_with(".ts"))
        .cloned()
        .collect();
    assert_eq!(
        declared_artifacts.len(),
        declared_sources.len(),
        "every asset needs one source and one artifact; declared={declared:?}"
    );
    let missing_from_rust: Vec<_> = declared_artifacts.difference(&embedded).cloned().collect();
    // And every embedded asset with a TS source must be declared, or its
    // artifact has no producer.
    let rust_js: BTreeSet<String> = embedded
        .iter()
        .filter(|p| p.ends_with(".js"))
        .cloned()
        .collect();
    let missing_from_manifest: Vec<_> = rust_js.difference(&declared_artifacts).cloned().collect();

    assert!(
        missing_from_rust.is_empty(),
        "these compiled artifacts are declared in tools/scripts/build-embedded-ts.ts but no \
         include_str! reads them (compiled for nothing, or the Rust site moved): {missing_from_rust:?}"
    );
    assert!(
        missing_from_manifest.is_empty(),
        "these files are include_str!-embedded but not produced by \
         tools/scripts/build-embedded-ts.ts (edit the .ts and declare it, never the .js): \
         {missing_from_manifest:?}"
    );

    // Every declared source exists in the tree, and its artifact is a sibling —
    // that adjacency is what makes `editing the .js` so tempting and what the
    // banner warns about.
    for source in &declared_sources {
        let path = repo_root().join(source);
        assert!(path.is_file(), "declared source {source} does not exist");
        let sibling: PathBuf = path.with_extension("js");
        assert!(
            sibling.is_file(),
            "declared source {source} has no emitted artifact beside it ({}) — \
             run node tools/scripts/build-embedded-ts.ts",
            sibling.display()
        );
    }
}
