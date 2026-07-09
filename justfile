set shell := ["bash", "-c"]
# On Windows just resolves recipe shebangs through the shell named here; without
# it just falls back to `cygpath`, which Git for Windows does not put on PATH.
set windows-shell := ["bash.exe", "-c"]
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
