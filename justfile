set shell := ["bash", "-c"]
default:
    @just --list
fmt:
    @find src -name '*.rs' -print0 | xargs -0 rustfmt --edition 2021
fmt-check:
    @find src -name '*.rs' -print0 | xargs -0 rustfmt --edition 2021 --check
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
    cargo run --release -- build docs --out target/site

ci: fmt-check && clippy && test
