set shell := ["bash", "-c"]
# Windows: Git Bash. Recipes use bash syntax (inline env vars, `2>/dev/null`,
# `command -v`), so pwsh would reject them. If `bash` resolves to WSL,
# `just windows-shell-check` reports the hijack and prints a PATH fix.
set windows-shell := ["bash.exe", "-c"]
set unstable
set lists

# Shared celestia-devtools recipes — NOT in git. Stage with: just fetch.
# `import?` silently skips when absent, so this justfile parses pre-fetch.
import? "./.just/git-bash-interop.just"
import? "./.just/celestia-devtools.just"

# Stage shared celestia-devtools recipes into .just/ (gitignored).
# Source order: explicit URL arg → local pip bundle (offline) → GitHub raw.
# curl honors HTTP_PROXY/HTTPS_PROXY/ALL_PROXY env vars automatically.
[script('bash')]
fetch URL='':
    #!/usr/bin/env bash
    set -euo pipefail
    out=.just/celestia-devtools.just
    mkdir -p .just
    if [ -n "{{URL}}" ]; then
      echo "[fetch] {{URL}} -> $out"
      curl -fsSL "{{URL}}" -o "$out"
    elif command -v celestia-devtools >/dev/null 2>&1; then
      src=$(celestia-devtools include-path)
      echo "[fetch] local bundle ($src) -> $out"
      cp "$src" "$out"
    else
      echo "[fetch] github raw -> $out"
      curl -fsSL "https://raw.githubusercontent.com/celestia-island/celestia-devtools/dev/src/celestia_devtools/common.just" -o "$out"
    fi
    echo "[fetch] wrote $out"

default:
    @just --list
fmt:
    cargo fmt -p lagrange-library
fmt-check:
    cargo fmt -p lagrange-library -- --check
check:
    cargo check --all-targets
clippy:
    cargo clippy --all-targets -- -D warnings
test:
    cargo test --all
build:
    cargo build --release

# Build lagrange's own documentation with lagrange itself (closed loop).
# Output goes to target/site/.
docs:
    cargo run --release -- build --src docs --out dist

# Build + watch: rebuilds the docs tree automatically on change.
# Self-contained [script] (not a linewise `just dev-watch` call) so it runs under
# the interop-pinned Git Bash even when WSL shadows PATH — malkuth + cargo are
# found because the whole body executes in one Git Bash process.
[script]
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    malkuth="{{malkuth_bin}}"
    if ! command -v "$malkuth" >/dev/null 2>&1 && [ ! -f "$malkuth" ]; then
      malkuth_path=$( {{python_cmd}} -m celestia_devtools locate --crate malkuth 2>/dev/null || true )
      if [ -n "$malkuth_path" ]; then
        suffix="target/release/malkuth"
        case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) suffix="target/release/malkuth.exe" ;; esac
        malkuth="$malkuth_path/$suffix"
      fi
    fi
    if ! command -v "$malkuth" >/dev/null 2>&1 && [ ! -f "$malkuth" ]; then
      echo "[dev] malkuth not found. Build it: cd ../malkuth && cargo build --release --features cli" >&2
      exit 1
    fi
    echo "[dev] supervising: cargo run --release dev --src docs --out dist --port 3000"
    echo "[dev] watching: docs src"
    exec "$malkuth" --watch docs --watch src --drain-secs 2 -- \
      cargo run --release -- dev --src docs --out dist --port 3000

ci: fmt-check clippy test
