set shell := ["bash", "-c"]
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
    cargo run --release -- build --src docs --out target/site

# Build + watch: rebuilds the docs tree automatically on change.
dev:
    cargo run --release -- dev --src docs --out target/site

ci: fmt-check && clippy && test
