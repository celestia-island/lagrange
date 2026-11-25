set shell := ["bash", "-c"]
# Windows: PowerShell (the 5.1 floor ships with every Windows; pwsh 7 is
# NOT assumed). Linewise recipes must stay PS-5.1-safe: no `&&` chains,
# `cd X; cmd` instead of `cd X && cmd`. Bash-only recipes use
# [script('bash')] and need Git Bash (or WSL) when actually run.
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command", "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $PSDefaultParameterValues['*:Encoding']='utf8';"]
set unstable
set lists

# Repo definitions override the shared template's (imported above).
set allow-duplicate-recipes
set allow-duplicate-variables

# Shared celestia-devtools recipes — NOT in git. Stage with: just fetch.
# `import?` silently skips when absent, so this justfile parses pre-fetch.
import? "./.just/celestia-devtools.just"

default:
    @just --list
fmt:
    just fmt-toml
    cargo fmt --all
fmt-check:
    cargo fmt --all -- --check
check: fmt-check clippy
    cargo check --workspace
clippy:
    cargo clippy --workspace --all-targets -- -D warnings
test:
    cargo test --workspace
build:
    cargo build --release

# ── embedded browser runtime ────────────────────────────────────────────────
# src/comments/*.ts and src/diagram.ts are the SOURCES of the six assets Rust
# embeds with include_str!; the .js beside each one is committed build output
# (cargo builds, and the crates.io tarball, consume the .js — see
# docs.yml / the hub's ci.yml, which never install this repo's toolchain).
# `just test` (cargo test) holds the two-sided drift lock:
# tests/embedded_ts.rs fails when an artifact stops matching the .ts, or when
# the manifest and the include_str! sites disagree.
#
# `embedded` also proves the compile is deterministic: it runs twice and
# compares sha256. Different esbuild VERSION between tools/ and packages/webui
# is a hard failure (one esbuild per tree); install/build are separate steps
# so the version guard is legible when it fires.
# [script('python')]: bash is banned from justfile recipes (#66).
[script('python')]
embedded:
    import hashlib, json, os, subprocess, sys

    root = os.path.dirname(os.path.abspath("justfile"))
    tools = os.path.join(root, "tools")
    manifest = os.path.join(tools, "package.json")
    declared = json.load(open(manifest, encoding="utf8"))["devDependencies"]["esbuild"]
    webui_manifest = os.path.join(root, "packages", "webui", "package.json")
    webui_declared = json.load(open(webui_manifest, encoding="utf8"))["devDependencies"]["esbuild"]
    if declared != webui_declared:
        sys.exit(
            f"embedded: tools/package.json declares esbuild {declared} but packages/webui "
            f"declares {webui_declared} — one esbuild per tree; make them equal."
        )
    if not os.path.isdir(os.path.join(tools, "node_modules", "esbuild")):
        print("embedded: tools/node_modules/esbuild missing — npm install --prefix tools (once)", file=sys.stderr)
        subprocess.run(["npm", "install", "--prefix", tools, "--no-audit", "--no-fund"], check=True)

    script = os.path.join(tools, "scripts", "build-embedded-ts.ts")

    def run():
        subprocess.run(["node", script], check=True)

    def digests():
        paths = [
            os.path.join(root, "src", "comments", name)
            for name in ("scrollbar.js", "modal.js", "clipboard.js", "ui.js", "runtime.js")
        ] + [os.path.join(root, "src", "diagram.js")]
        return {
            os.path.relpath(p, root): hashlib.sha256(open(p, "rb").read()).hexdigest() for p in paths
        }

    run()
    first = digests()
    run()
    second = digests()
    if first != second:
        changed = [p for p in first if first[p] != second[p]]
        sys.exit(f"embedded: NOT deterministic — these artifacts differ between two runs: {changed}")
    print(f"embedded: {len(first)} artifacts, byte-identical across two runs (sha256)")
    for path, digest in sorted(first.items()):
        print(f"  {digest}  {path}")

# Build lagrange's own documentation with lagrange itself (closed loop).
# Output goes to dist/. The hikari webui lane (per-language pages on the
# real hikari component tree) is built first when pnpm is available;
# without it the build warns and keeps the legacy chrome.
# [script('python')]: bash is banned from justfile recipes (#66).
[script('python')]
docs:
    import os, shutil, subprocess, sys

    webui = "packages/webui"
    if shutil.which("pnpm") and os.path.isdir(webui):
        subprocess.run(["pnpm", "--dir", webui, "install", "--no-frozen-lockfile"], check=True)
        subprocess.run(["pnpm", "--dir", webui, "build:docs:all"], check=True)
    else:
        print("docs: pnpm or packages/webui missing — legacy chrome only", file=sys.stderr)
    env = dict(os.environ, LAGRANGE_SKIP_LIVE="1")
    subprocess.run(["cargo", "run", "--release", "--", "build", "--src", "docs", "--out", "dist"], check=True, env=env)

# Just the webui docs bundles (hydration client + Node SSR renderer).
docs-webui:
    pnpm --dir packages/webui install --no-frozen-lockfile
    pnpm --dir packages/webui build:docs:all

# Build + watch: rebuilds the docs tree automatically on change.
# malkuth path is resolved by just at parse time ({{malkuth_bin}}); the body
# only checks usability and falls back to the sibling-repo release binary.
[script('python')]
dev:
    import os, shutil, sys
    from datetime import datetime

    def log(msg):
        print(f"{datetime.now():%Y-%m-%d %H:%M:%S}  INFO lagrange-dev: {msg}")

    def err(msg):
        print(f"{datetime.now():%Y-%m-%d %H:%M:%S} ERROR lagrange-dev: {msg}", file=sys.stderr)

    # Resolve malkuth: prefer {{malkuth_bin}}, fall back to sibling-repo release.
    malkuth = r"{{malkuth_bin}}"
    if shutil.which(malkuth) is None and not os.path.isfile(malkuth):
        malkuth = "../malkuth/target/release/malkuth.exe"
    # Final check: usable as command OR exists as a file.
    if shutil.which(malkuth) is None and not os.path.isfile(malkuth):
        err("malkuth not found. Build it: cd ../malkuth && cargo build --release --features cli")
        err("or set: export MALKUTH_BIN=/path/to/malkuth")
        sys.exit(1)
    log("supervising: cargo run --release dev --src docs --out dist --port 3000")
    log("watching: docs src")
    os.execvp(malkuth, [malkuth, "--watch", "docs", "--watch", "src", "--drain-secs", "2", "--",
                        "cargo", "run", "--release", "--", "dev", "--src", "docs", "--out", "dist", "--port", "3000"])

ci: fmt-check clippy test

