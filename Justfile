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

default:
    @just --list

build:
    cargo +{{toolchain}} build --all

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

examples:
    #!/usr/bin/env bash
    set -euo pipefail
    for path in examples/*.rs; do
        cargo +{{toolchain}} run --example "$(basename "${path}" .rs)" >/dev/null
    done

lint:
    cargo +{{stable}} clippy --workspace --lib --bins --examples --tests --all-features -- -D warnings
    cargo +{{stable}} clippy --workspace --lib --bins --examples --tests --no-default-features -- -D warnings
    cargo +{{stable}} fmt --all -- --check
    cargo +{{stable}} fmt --manifest-path example_no_std/Cargo.toml -- --check
    cargo +{{stable}} fmt --manifest-path example_wasm/Cargo.toml -- --check
    cargo +{{stable}} clippy --manifest-path example_no_std/Cargo.toml --lib -- -D warnings
    cargo +{{stable}} clippy --manifest-path example_wasm/Cargo.toml --lib -- -D warnings

no-std target:
    cd example_no_std && cargo +{{toolchain}} build --release --target {{target}}
    cd example_no_std && cargo +{{toolchain}} build --release --target {{target}} --no-default-features

no-std-v7:
    just no-std thumbv7em-none-eabihf

no-std-v6:
    just no-std thumbv6m-none-eabi

wasm:
    cd example_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack build --target web
    cd example_wasm && RUSTUP_TOOLCHAIN={{toolchain}} wasm-pack test --node

ci: build test examples no-std-v7 no-std-v6 wasm

ci-msrv:
    DEKU_TOOLCHAIN=msrv just ci

ci-stable:
    DEKU_TOOLCHAIN=stable just ci

ci-lint:
    DEKU_TOOLCHAIN=stable just lint

ci-beta:
    DEKU_TOOLCHAIN=beta just ci

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

bench:
    cargo +{{toolchain}} bench --workspace
