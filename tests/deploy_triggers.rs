//! The docs deploy workflow's trigger contract, as a gate.
//!
//! `docs.yml` redeploys the public site on `push` — but only when a push
//! touches a path it lists. Every file that changes the DEPLOYED BYTES
//! must therefore be listed, and every listed path must be able to
//! produce a tracked-diff event.
//!
//! MUT: decoy-paths (deleting a required path entry fails the coverage
//! test; a decoy block above a gutted `push.paths` fails it too).
//! MUT: branches-absent · MUT: filters-key.
//!
//! This gate has been broken three times by reviewers, and each break
//! taught the same lesson: a contract check that can be satisfied while
//! the deploy never fires is worse than none, because it disarms the
//! human who would have read the file. So it parses with a real YAML
//! parser, reads ONLY `on.push`, and **fails closed** on every shape it
//! does not explicitly understand *within that trigger block* (unknown
//! keys, absent `branches`, `paths-ignore`).
//!
//! Scope, stated so nobody over-reads the green check: this proves the
//! PUSH TRIGGER. It cannot see a job-level `if:` that skips the deploy,
//! nor anything GitHub does at runtime — that stays a human review.
//!
//! YAML anchors/aliases are deliberately NOT scanned here (PR #120
//! residual cleanup, closing rounds 8/9's simplification
//! recommendation): `serde_yaml` resolves aliases before this gate runs,
//! so the coverage test below already validates whatever the file
//! resolves to — a list too short fails, a list spelled through an
//! anchor passes only if it genuinely contains every required input.
//! GitHub's loader (anchors supported since 2025-09) honours the same
//! expansion, so a correctly-anchored list deploys exactly what the
//! coverage test validated; an over-short one fails HERE, before merge.
//!

use serde_yaml::Value;

fn workflow_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/docs.yml")
}

fn workflow() -> Value {
    let raw = std::fs::read_to_string(workflow_path()).expect("read docs.yml");
    serde_yaml::from_str(&raw).expect("docs.yml must be valid YAML")
}

/// The `on.push` mapping of a parsed workflow, or a panic naming the
/// shape that was not understood — never a silent skip.
/// MUT: decoy-paths · MUT: branches-absent · MUT: filters-key.
fn push_trigger_from(doc: &Value) -> serde_yaml::Mapping {
    let on = doc
        .get("on")
        .or_else(|| doc.get("true")) // YAML 1.1 parses a bare `on:` key as `true`
        .expect("`on:` must declare an `on:` block");

    let push = on
        .get("push")
        .unwrap_or_else(|| panic!("`on:` must declare a `push` event; got {on:?}"));
    let mapping = push
        .as_mapping()
        .unwrap_or_else(|| panic!("`on.push` must be a mapping, got {push:?}"));

    // Fail closed on ANY key this contract does not understand. An unknown
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

/// The production reader: the real file's `on.push`.
fn push_trigger() -> serde_yaml::Mapping {
    push_trigger_from(&workflow())
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
///
/// `src/**` also covers the TypeScript sources of the embedded browser
/// runtime (`src/comments/*.ts`, `src/diagram.ts`): editing one and running
/// `just embedded` changes the emitted `.js` the binary embeds, so a source
/// change ships different bytes. `tools/**` holds the compiler that turns
/// them into the embedded `.js` — a change there changes the artifacts on the
/// next regen, so it must redeploy for the same reason.
const REQUIRED: &[(&str, &str)] = &[
    ("docs/**", "content, SUMMARY, site config, logo"),
    (
        "src/**",
        "renderer, site builder, embedded JS/assets (incl. the .ts sources of the embedded runtime)",
    ),
    (
        "tools/**",
        "the compiler that emits the embedded runtime .js from its .ts sources",
    ),
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
    // `paths:` matches changed TRACKED files, so a gitignored path listed
    // there reads like a guarantee while delivering nothing — review round
    // 4 of PR #120.
    //
    // The webui lockfile is one of those: `packages/webui/.gitignore` ignores
    // it, so it is never tracked (verify with `git ls-files packages/webui | grep lock`
    // — which returns nothing; `create-lagrange/pnpm-lock.yaml`, a different
    // package, IS tracked). What resolves lagrange's webui dependency graph is
    // `packages/webui/pnpm-workspace.yaml` (exact release-age pins), which this
    // trigger list does carry.
    let triggers = deploy_triggers();
    assert!(
        !triggers.iter().any(|t| t.contains("pnpm-lock")),
        "packages/webui/pnpm-lock.yaml is gitignored (packages/webui/.gitignore) \
         and never tracked, so it can never produce a tracked-diff event; do not \
         list it as a deploy trigger. Parsed: {triggers:?}"
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
                "packages/webui/pnpm-lock.yaml is no longer gitignored — it is \
                 now tracked and can trigger deploys, so revisit the trigger \
                 list and the --no-frozen-lockfile choice"
            );
        }
    }
}

/// The parser must read `on.push` — never "the first `paths:` in the
/// file" (round 5's decoy attack drove the production function, not a
/// hand-built literal; PR #120 residual cleanup gave this test the
/// production path after round 9 proved the old shape could pass
/// vacuously).
/// MUT: decoy-paths.
#[test]
fn the_parser_reads_only_the_push_event() {
    let decoy: Value = serde_yaml::from_str(
        // The decoy's pull_request block mirrors the real file's shape
        // (branches + paths), so a regression reaches the paths assertion
        // instead of tripping the branches check first (round 2).
        "on:\n  pull_request:\n    branches: [master]\n    paths:\n      - 'docs/**'\n  push:\n    branches: [master]\n    paths:\n      - 'src/**'\n",
    )
    .unwrap();
    let mapping = push_trigger_from(&decoy);
    let push_paths: Vec<&str> = mapping["paths"]
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
