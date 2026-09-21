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
import? "./.just/git-bash-interop.just"
import? "./.just/celestia-devtools.just"

# Stage shared celestia-devtools recipes into .just/ (gitignored).
# Source order: explicit URL arg → local pip bundle (offline) → GitHub raw.
# curl honors HTTP_PROXY/HTTPS_PROXY/ALL_PROXY env vars automatically.
fetch URL='':
    {{ if os_family() == "windows" { "python" } else { "python3" } }} -c "import os; os.makedirs('.just', exist_ok=True)"
    {{ if URL != "" { "curl -fsSL " + URL + " -o .just/celestia-devtools.just" } else if which("celestia-devtools") != "" { "celestia-devtools fetch-just" } else { "curl -fsSL https://raw.githubusercontent.com/celestia-island/celestia-devtools/dev/src/celestia_devtools/common.just -o .just/celestia-devtools.just" } }}
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

# Build lagrange's own documentation with lagrange itself (closed loop).
# Output goes to target/site/.
docs:
    cargo run --release -- build --src docs --out dist

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

