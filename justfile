# Use Git Bash explicitly — on Windows, `bash` may resolve to WSL which lacks cargo.
# The `windows-shell` setting takes precedence on Windows hosts.
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-c"]
# Fallback for non-Windows: use system bash.
set shell := ["bash", "-c"]
# `set lists` enables which() (used by the imported celestia-devtools.just);
# `set unstable` gates it.
set unstable
set lists
import "./celestia-devtools.just"

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
dev:
    cargo run --release -- dev --src docs --out dist --port 0

ci: fmt-check clippy test
