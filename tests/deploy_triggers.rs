//! The docs deploy workflow's trigger contract, as a gate.
//!
//! `docs.yml` redeploys the public site on `push` — but only when a push
//! touches a path it lists. Every file that changes the DEPLOYED BYTES
//! must therefore be listed, and every listed path must be able to
//! produce a tracked-diff event. Both halves failed during review of
//! PR #120 (a compiled-in stylesheet missing; a gitignored lockfile
//! listed as if it could trigger).
//!
//! The first version of this gate hand-parsed the file and locked onto
//! the FIRST `paths:` it saw — so a decoy `pull_request: paths:` block
//! above `push:` satisfied it while `push.paths` was gutted, and
//! `paths-ignore:` / `branches:` were never read. A gate that can be
//! satisfied while the deploy never fires is worse than no gate: it
//! stops the reviewer who would otherwise read the file. This version
//! parses the YAML with a real parser and **fails closed** on every
//! shape it does not explicitly understand.

use serde_yaml::Value;

fn workflow() -> Value {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/docs.yml"),
    )
    .expect("read docs.yml");
    serde_yaml::from_str(&raw).expect("docs.yml must be valid YAML")
}

/// The trigger block of the `push` event, or a panic naming the shape
/// that was not understood (never a silent skip).
fn push_trigger() -> serde_yaml::Mapping {
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

    // Fail closed on anything that can turn the trigger off while leaving
    // `paths:` intact.
    if mapping.contains_key("paths-ignore") {
        panic!(
            "`on.push.paths-ignore` combined with `paths` silently disables triggers; remove it"
        );
    }
    if let Some(branches) = mapping.get("branches") {
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
    }
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
/// (`tairitsu_macros::scss!` in src/theme.rs); the workspace file decides
/// which hikari resolves for the SSR renderer; the vite config owns the
/// output shape and the `/assets/docs/` base.
const REQUIRED: &[(&str, &str)] = &[
    ("docs/**", "content, SUMMARY, site config, logo"),
    ("src/**", "renderer, site builder, embedded JS/assets"),
    ("styles/**", "scss!-embedded chrome"),
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
    // trigger list and the --frozen-lockfile choice must be revisited.
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

/// The gate must reject the shapes that can disable the deploy while
/// leaving a happy-looking `paths:` list (review round 5's S1/S2): a
/// decoy block in another event, `paths-ignore`, and a wrong branch.
#[test]
fn the_gate_fails_closed_on_contract_breaking_shapes() {
    // A decoy `paths:` in another event must not be mistaken for the
    // push trigger: `deploy_triggers()` reads `on.push.paths` only.
    let decoy = "on:\n  pull_request:\n    paths:\n      - 'docs/**'\n  push:\n    branches: [master]\n    paths:\n      - 'src/**'\n";
    let doc: Value = serde_yaml::from_str(decoy).unwrap();
    let on = doc.get("on").unwrap();
    let push_paths: Vec<&str> = on["push"]["paths"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    assert_eq!(
        push_paths,
        vec!["src/**"],
        "the parser must read push.paths, never the first `paths:` in the file"
    );
}
