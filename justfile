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
# Self-contained [script]: malkuth path is resolved by just at parse time
# ({{malkuth_bin}}), so the body needs no uname/python/command-v — only bash
# builtins ([ -f ], exec, echo), which work even with a stripped-down PATH.
[script]
dev:
    #!/usr/bin/env bash
    set -eu
    # Ensure MSYS /usr/bin is on PATH (just spawns bash.exe without /etc/profile).
    case ":$PATH:" in
      *":/usr/bin:"*) ;;
      *) PATH="/usr/bin:$PATH" ;;
    esac
    # tracing-style log helper — matches lagrange_library's own format:
    # local time, "%Y-%m-%d %H:%M:%S", no T/Z (see src/main.rs Timer impl).
    log() { printf '%s  INFO lagrange-dev: %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"; }
    err() { printf '%s ERROR lagrange-dev: %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >&2; }
    # Resolve malkuth: prefer {{malkuth_bin}}, fall back to sibling-repo release.
    malkuth="{{malkuth_bin}}"
    if ! command -v "$malkuth" >/dev/null 2>&1 && [ ! -f "$malkuth" ]; then
      malkuth="../malkuth/target/release/malkuth.exe"
    fi
    if [ ! -f "$malkuth" ]; then
      err "malkuth not found. Build it: cd ../malkuth && cargo build --release --features cli"
      err "or set: export MALKUTH_BIN=/path/to/malkuth"
      exit 1
    fi
    log "supervising: cargo run --release dev --src docs --out dist --port 3000"
    log "watching: docs src"
    exec "$malkuth" --watch docs --watch src --drain-secs 2 -- \
      cargo run --release -- dev --src docs --out dist --port 3000

ci: fmt-check clippy test

