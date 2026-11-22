//! The docs deploy workflow's trigger contract, as a gate.
//!
//! `docs.yml` redeploys the public site on `push` — but only when a push
//! touches a path it lists. Every file that changes the DEPLOYED BYTES
//! must therefore be listed, and every listed path must be able to
//! produce a tracked-diff event. Both halves failed in review rounds
//! 2–4 of PR #120 (a compiled-in stylesheet missing from the list; a
//! gitignored lockfile listed as if it could trigger). Reviewers caught
//! what no gate could — so this test is the gate.

/// Parse the `paths:` entries of the workflow's `on.push` block.
fn deploy_triggers() -> Vec<String> {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/docs.yml"),
    )
    .expect("read docs.yml");

    let mut out = Vec::new();
    let mut in_paths = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "paths:" {
            in_paths = true;
            continue;
        }
        if !in_paths {
            continue;
        }
        // A new top-level key (or a section like `workflow_dispatch:`)
        // ends the list: those lines carry no leading dash.
        if !trimmed.starts_with('-') {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            break;
        }
        out.push(
            trimmed
                .trim_start_matches('-')
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string(),
        );
    }
    out
}

#[test]
fn deploy_triggers_cover_every_input_that_changes_the_shipped_site() {
    let triggers = deploy_triggers();
    assert!(
        !triggers.is_empty(),
        "no push paths parsed from docs.yml — the parser or the file changed shape"
    );

    // Each entry is an input whose change alters the deployed bytes, with
    // the reason it cannot be dropped. `styles/**` is compiled INTO the
    // binary (`tairitsu_macros::scss!` in src/theme.rs); the workspace
    // file decides which hikari resolves for the SSR renderer; the vite
    // config owns the output shape and the /assets/docs/ base.
    for required in [
        "docs/**",                            // content, SUMMARY, site config, logo
        "src/**",                             // renderer, site builder, embedded JS
        "styles/**",                          // scss!-embedded chrome
        "packages/webui/src/**",              // shell components + hydration entry
        "packages/webui/scripts/**",          // the Node SSR renderer
        "packages/webui/package.json",        // dependency constraints
        "packages/webui/pnpm-workspace.yaml", // release-age → which hikari
        "packages/webui/vite.config.ts",      // outDir, base, SSR entry
        "Cargo.toml",                         // crate deps / features
        ".github/workflows/docs.yml",         // this workflow itself
    ] {
        assert!(
            triggers.iter().any(|t| t == required),
            "docs.yml push paths must include `{required}` — a change there ships \
             different bytes but would not redeploy. Parsed: {triggers:?}"
        );
    }
}

#[test]
fn deploy_triggers_never_list_an_untracked_path() {
    // `paths:` matches changed TRACKED files. The webui lockfile is
    // gitignored (packages/webui/.gitignore), so listing it reads like a
    // guarantee while delivering nothing — review round 4 of PR #120.
    let triggers = deploy_triggers();
    assert!(
        !triggers.iter().any(|t| t.contains("pnpm-lock")),
        "pnpm-lock.yaml is gitignored and can never produce a tracked-diff \
         event; do not list it as a deploy trigger. Parsed: {triggers:?}"
    );

    // Guard the premise itself: if the lockfile ever becomes tracked, the
    // assertion above must be revisited together with --frozen-lockfile.
    let ignored = std::process::Command::new("git")
        .args(["check-ignore", "-q", "packages/webui/pnpm-lock.yaml"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status();
    if let Ok(status) = ignored {
        // Only assert when git is available and reports an answer; a
        // checkout without git (source tarball) skips the premise check.
        if status.code().is_some() {
            assert!(
                status.success(),
                "packages/webui/pnpm-lock.yaml is no longer gitignored — it can \
                 now trigger deploys, so revisit the trigger list and the \
                 --no-frozen-lockfile choice"
            );
        }
    }
}
