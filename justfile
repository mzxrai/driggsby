set shell := ["zsh", "-cu"]

required-check:
    cargo clippy --all-targets --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::undocumented_unsafe_blocks

rust-verify:
    cargo fmt --all
    just required-check
    cargo test --all-features
    cargo build

security-bootstrap-macos:
    bash scripts/security/bootstrap-macos.sh

security-local:
    bash scripts/security/local-scan.sh --mode full
