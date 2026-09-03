set shell := ["bash", "-euc"]

stable := env_var_or_default("DEKU_STABLE_TOOLCHAIN", "stable")
toolchain := env_var_or_default("DEKU_TOOLCHAIN", "stable")

# Non-default feature combinations exercised by the test and coverage matrix.
feature_matrix := "std alloc descriptive-errors bits logging bits,alloc"

# List the available build commands.
default:
    @just --list

# Compile the complete workspace with the selected toolchain.
build:
    cargo +{{toolchain}} build --all

# Run the test matrix used by CI.
test:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo +{{toolchain}} test --all
    cargo +{{toolchain}} test --all-features
    cargo +{{toolchain}} test --no-default-features
    for features in {{feature_matrix}}; do
        cargo +{{toolchain}} test --no-default-features --features="${features}"
    done

# Run every example with the default feature set.
examples:
    #!/usr/bin/env bash
    set -euo pipefail
    for path in examples/*.rs; do
        cargo +{{toolchain}} run --example "$(basename "${path}" .rs)" >/dev/null
    done

# Run the build, test, and example checks for the selected toolchain.
ci-core: build test examples

# Run formatting and Clippy checks with stable Rust.
lint:
    cargo +{{stable}} clippy --workspace --lib --bins --examples --tests --all-features -- -D warnings
    cargo +{{stable}} clippy --workspace --lib --bins --examples --tests --no-default-features -- -D warnings
    cargo +{{stable}} fmt --all -- --check

# Compile the no_std fixture for a target.
no-std target:
    cd ensure_no_std && cargo +{{toolchain}} build --release --target {{target}}
    cd ensure_no_std && cargo +{{toolchain}} build --release --target {{target}} --no-default-features

# Run the thumbv7em no_std checks.
no-std-v7:
    just no-std thumbv7em-none-eabihf

# Run the thumbv6m no_std checks.
no-std-v6:
    just no-std thumbv6m-none-eabi

# Build and test the WebAssembly fixture.
wasm:
    cd ensure_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack build --target web
    cd ensure_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack test --node

# Run the build, test, example, no_std, and WebAssembly checks for the selected toolchain.
ci: ci-core no-std-v7 no-std-v6 wasm

# Run the Criterion benchmarks.
bench:
    cargo +{{toolchain}} bench --workspace
