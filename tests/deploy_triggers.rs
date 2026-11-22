//! The docs deploy workflow's trigger contract, as a gate.
//!
//! `docs.yml` redeploys the public site on `push` — but only when a push
//! touches a path it lists. Every file that changes the DEPLOYED BYTES
//! must therefore be listed, and every listed path must be able to
//! produce a tracked-diff event.
//!
//! This gate has been broken three times by reviewers, and each break
//! taught the same lesson: a contract check that can be satisfied while
//! the deploy never fires is worse than none, because it disarms the
//! human who would have read the file. So it parses with a real YAML
//! parser, reads ONLY `on.push`, and **fails closed** on every shape it
//! does not explicitly understand *within that trigger block* (unknown
//! keys, absent `branches`, `paths-ignore`, YAML anchors/aliases).
//!
//! MUT: decoy-paths (a `pull_request: paths:` block above a gutted
//! `push.paths` used to pass — round 5), MUT: branches-absent, MUT: alias-paths.
//!
//! Scope, stated so nobody over-reads the green check: this proves the
//! PUSH TRIGGER. It cannot see a job-level `if:` that skips the deploy,
//! nor anything GitHub does at runtime — that stays a human review.

use serde_yaml::Value;

fn workflow_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/docs.yml")
}

fn workflow() -> Value {
    let raw = std::fs::read_to_string(workflow_path()).expect("read docs.yml");
    serde_yaml::from_str(&raw).expect("docs.yml must be valid YAML")
}

/// Drop a trailing YAML comment. `#` only opens a comment when it starts a
/// token, so `foo#bar` keeps its bytes (and a quoted `'#'` is left alone —
/// this file has none, and being conservative here only means a stricter
/// alias scan, never a looser one).
/// MUT: alias-paths (an anchor/alias in value position still fails the gate).
fn strip_yaml_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut prev_is_space = true;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'#' && prev_is_space {
            return &line[..i];
        }
        prev_is_space = b.is_ascii_whitespace();
    }
    line
}

/// The `on.push` mapping, or a panic naming the shape that was not
/// understood — never a silent skip.
/// MUT: branches-absent · MUT: filters-key · MUT: decoy-paths.
fn push_trigger() -> serde_yaml::Mapping {
    // YAML anchors/aliases are expanded before this code ever sees them, so
    // a `paths: *all` could point at a list defined under a decoy event.
    // Refuse the construct — but only where YAML can actually place one
    // (value position after `key:` or a `- ` item), because a blanket scan
    // for `&` false-fails on the `if: a && b` idiom three of this repo's
    // own workflows use (review round 7 of PR #120).
    let raw = std::fs::read_to_string(workflow_path()).expect("read docs.yml");
    let has_alias = raw.lines().any(|line| alias_token(line).is_some());
    assert!(
        !has_alias,
        "YAML anchors/aliases in docs.yml defeat this gate's ability to prove what \
         `on.push.paths` contains; spell the list out"
    );

    let doc = workflow();
    let on = doc
        .get("on")
        .or_else(|| doc.get("true")) // YAML 1.1 parses a bare `on:` key as `true`
        .expect("docs.yml must declare an `on:` block");

    let push = on
        .get("push")
        .unwrap_or_else(|| panic!("`on:` must declare a `push` event; got {on:?}"));
    let mapping = push
        .as_mapping()
        .unwrap_or_else(|| panic!("`on.push` must be a mapping, got {push:?}"));

    // Fail closed on ANY key this contract does not understand. An unknown
    // MUT: filters-key (an unknown `on.push` key refuses to pass).
    // key (a nested `filters:` block, a future GitHub addition) can
    // disable the deploy while `paths:` still looks complete — round 6
    // found three such shapes passing a whitelist-less check.
    for key in mapping.keys() {
        let name = key.as_str().unwrap_or("<non-string key>");
        assert!(
            matches!(
                name,
                "branches" | "branches-ignore" | "paths" | "paths-ignore" | "tags" | "tags-ignore"
            ),
            "unknown `on.push` key `{name}` — the gate cannot prove it is harmless, so it \
             refuses to pass. Add it to the whitelist with a reason."
        );
    }

    // `paths-ignore` combined with `paths` narrows the trigger; alone it
    // inverts it. Either way it can silently kill deploys.
    assert!(
        !mapping.contains_key("paths-ignore"),
        "`on.push.paths-ignore` silently narrows or inverts the trigger; remove it"
    );
    assert!(
        !mapping.contains_key("branches-ignore"),
        "`on.push.branches-ignore` is not part of this deploy contract"
    );

    // An ABSENT `branches` means "any ref, tags included" — not the
    // master-only deploy this repo intends (round 6 bypass 3b).
    let branches = mapping
        .get("branches")
        .unwrap_or_else(|| panic!("`on.push.branches` is required (must include `master`)"));
    let list: Vec<&str> = branches
        .as_sequence()
        .unwrap_or_else(|| panic!("`on.push.branches` must be a list, got {branches:?}"))
        .iter()
        .filter_map(|b| b.as_str())
        .collect();
    assert!(
        list.contains(&"master"),
        "`on.push.branches` must include `master` (the trunk this repo deploys from); got {list:?}"
    );

    mapping.clone()
}

fn deploy_triggers() -> Vec<String> {
    let push = push_trigger();
    let paths = push
        .get("paths")
        .unwrap_or_else(|| panic!("`on.push.paths` is required; the deploy would never fire"));
    let list = paths
        .as_sequence()
        .unwrap_or_else(|| panic!("`on.push.paths` must be a list, got {paths:?}"));
    let triggers: Vec<String> = list
        .iter()
        .map(|p| {
            p.as_str()
                .unwrap_or_else(|| panic!("every `paths` entry must be a string, got {p:?}"))
                .to_string()
        })
        .collect();
    assert!(!triggers.is_empty(), "`on.push.paths` must not be empty");
    triggers
}

/// Every input whose change alters the deployed bytes, with the reason it
/// cannot be dropped. `styles/**` is compiled INTO the binary
/// (`tairitsu_macros::scss!` in src/theme.rs) and is what the LEGACY lane
/// inlines; the workspace file decides which hikari resolves for the SSR
/// renderer; the vite config owns the output shape and the
/// `/assets/docs/` base; `LICENSE` is copied into the output by
/// `copy_root_assets`.
const REQUIRED: &[(&str, &str)] = &[
    ("docs/**", "content, SUMMARY, site config, logo"),
    ("src/**", "renderer, site builder, embedded JS/assets"),
    (
        "styles/**",
        "scss!-embedded chrome (legacy lane inlines it)",
    ),
    (
        "packages/webui/src/**",
        "shell components + hydration entry",
    ),
    ("packages/webui/scripts/**", "the Node SSR renderer"),
    ("packages/webui/package.json", "dependency constraints"),
    (
        "packages/webui/pnpm-workspace.yaml",
        "release-age settings decide WHICH hikari ships",
    ),
    (
        "packages/webui/vite.config.ts",
        "outDir, the /assets/docs/ base, the SSR entry",
    ),
    ("Cargo.toml", "crate deps / features"),
    ("LICENSE", "copied into the output by copy_root_assets"),
    (".github/workflows/docs.yml", "this workflow itself"),
];

#[test]
fn deploy_triggers_cover_every_input_that_changes_the_shipped_site() {
    let triggers = deploy_triggers();
    for (required, why) in REQUIRED {
        assert!(
            triggers.iter().any(|t| t == required),
            "docs.yml push paths must include `{required}` ({why}) — a change there \
             ships different bytes but would not redeploy. Parsed: {triggers:?}"
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

    // Guard the premise: if the lockfile ever becomes tracked, both the
    // trigger list and the --no-frozen-lockfile choice must be revisited.
    let ignored = std::process::Command::new("git")
        .args(["check-ignore", "-q", "packages/webui/pnpm-lock.yaml"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status();
    if let Ok(status) = ignored {
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

/// The parser must read `on.push`, never "the first `paths:` in the file"
/// (round 5's decoy attack) and never an alias-defined list (round 6's).
/// MUT: decoy-paths · MUT: alias-paths.
#[test]
fn the_parser_reads_only_the_push_event() {
    let decoy = "on:\n  pull_request:\n    paths:\n      - 'docs/**'\n  push:\n    branches: [master]\n    paths:\n      - 'src/**'\n";
    let doc: Value = serde_yaml::from_str(decoy).unwrap();
    let push_paths: Vec<&str> = doc["on"]["push"]["paths"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    assert_eq!(
        push_paths,
        vec!["src/**"],
        "the contract must read push.paths, never the first `paths:` in the file"
    );
}

/// The alias scan must not fire on code that merely CONTAINS `&` — the
/// repo's own workflows gate jobs with `if: … && …` (review round 7 of
/// PR #120: a blanket `contains('&')` would have rejected that idiom).
#[test]
fn the_alias_scan_ignores_operators_and_comments() {
    for innocent in [
        "    if: github.event_name == 'push' && !github.event.deleted",
        "      run: cargo build 2>&1",
        "    # paths: *all would be rejected, but this is only a comment",
        "      - 'docs/**'  # not & an anchor",
    ] {
        assert_eq!(
            alias_token(innocent),
            None,
            "`{innocent}` must not be read as a YAML alias/anchor"
        );
    }

    for offensive in [
        "    paths: &all",
        "    paths: *all",
        "      - &entry 'docs/**'",
        "      - *entry",
    ] {
        assert!(
            alias_token(offensive).is_some(),
            "`{offensive}` IS a YAML anchor/alias and must be rejected"
        );
    }
}

/// The value-position anchor/alias detector, split out so the tests above
/// can drive it directly (the production path uses the same code).
fn alias_token(line: &str) -> Option<&str> {
    let line = strip_yaml_comment(line);
    let t = line.trim();
    let value = t
        .split_once(':')
        .map(|(_, v)| v.trim())
        .unwrap_or_else(|| t.strip_prefix('-').map(str::trim).unwrap_or(""));
    if value.starts_with('&') || value.starts_with('*') {
        Some(value)
    } else {
        None
    }
}
