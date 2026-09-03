set shell := ["bash", "-euc"]

stable := env_var_or_default("DEKU_STABLE_TOOLCHAIN", "stable")
msrv := env_var_or_default("DEKU_MSRV_TOOLCHAIN", "1.88.0")
beta := env_var_or_default("DEKU_BETA_TOOLCHAIN", "beta")
nightly := env_var_or_default("DEKU_NIGHTLY_TOOLCHAIN", "nightly")
pipeline := env_var_or_default("DEKU_TOOLCHAIN", "stable")
toolchain := if pipeline == "msrv" { msrv } else if pipeline == "stable" { stable } else if pipeline == "beta" { beta } else { pipeline }
trybuild_profile := if pipeline == "msrv" { "msrv" } else if pipeline == "beta" { "beta" } else { "stable" }

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
    export DEKU_TRYBUILD_PROFILE="{{trybuild_profile}}"
    cargo +{{toolchain}} test --all
    cargo +{{toolchain}} test --all-features
    cargo +{{toolchain}} test --no-default-features
    for features in {{feature_matrix}}; do
        cargo +{{toolchain}} test --no-default-features --features="${features}"
    done

# Run the same test matrix under Miri.
miri-test:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo +{{nightly}} miri setup
    cargo +{{nightly}} miri test --workspace
    cargo +{{nightly}} miri test --workspace --all-features
    cargo +{{nightly}} miri test --workspace --no-default-features
    for features in {{feature_matrix}}; do
        cargo +{{nightly}} miri test --workspace --no-default-features --features="${features}"
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
    cd example_no_std && cargo +{{toolchain}} build --release --target {{target}}
    cd example_no_std && cargo +{{toolchain}} build --release --target {{target}} --no-default-features

# Run the thumbv7em no_std checks.
no-std-v7:
    just no-std thumbv7em-none-eabihf

# Run the thumbv6m no_std checks.
no-std-v6:
    just no-std thumbv6m-none-eabi

# Build and test the WebAssembly fixture.
wasm:
    cd example_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack build --target web
    cd example_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack test --node

# Run the build, test, example, no_std, and WebAssembly checks for the selected toolchain.
ci: ci-core no-std-v7 no-std-v6 wasm

# Run the complete CI pipeline with a specific toolchain.
ci-msrv:
    DEKU_TOOLCHAIN=msrv just ci

ci-stable:
    DEKU_TOOLCHAIN=stable just ci

ci-lint:
    DEKU_TOOLCHAIN=stable just lint

ci-beta:
    DEKU_TOOLCHAIN=beta just ci

# Generate one Codecov JSON report from the complete feature test matrix.
coverage:
    #!/usr/bin/env bash
    set -euo pipefail

    # Discard stale execution data while retaining instrumented build artifacts.
    cargo +{{stable}} llvm-cov clean --profraw-only

    cargo +{{stable}} llvm-cov --workspace --no-report
    cargo +{{stable}} llvm-cov --workspace --all-features --no-report
    cargo +{{stable}} llvm-cov --package deku --no-default-features --no-report
    for features in {{feature_matrix}}; do
        cargo +{{stable}} llvm-cov --package deku --no-default-features --features="${features}" --no-report
    done

    cargo +{{stable}} llvm-cov report --package deku --package deku_derive --codecov --output-path codecov.json

# Run the Criterion benchmarks.
bench:
    cargo +{{toolchain}} bench --workspace

# Run the workspace test suite under Miri.
# Miri requires the nightly toolchain and its rust-src component.
miri:
    rustup component add miri rust-src --toolchain nightly
    cargo +nightly miri setup
    cargo +nightly miri test --workspace --all-features
