# syntax=docker/dockerfile:1.7

FROM node:24-bookworm-slim AS node-runtime

FROM rust:1.88-slim-bookworm

ARG STABLE_TOOLCHAIN=stable
ARG MSRV_TOOLCHAIN=1.88.0
ARG BETA_TOOLCHAIN=beta
ARG JUST_VERSION=1.58.0
ARG WASM_PACK_VERSION=0.15.0
ARG CARGO_LLVM_COV_VERSION=0.8.7
ARG USER_UID=1000
ARG USER_GID=1000

COPY --from=node-runtime /usr/local/bin/node /usr/local/bin/node

RUN rustup toolchain install "${MSRV_TOOLCHAIN}" --profile minimal \
    && rustup toolchain install "${STABLE_TOOLCHAIN}" --profile minimal \
    && rustup toolchain install "${BETA_TOOLCHAIN}" --profile minimal \
    && for toolchain in "${MSRV_TOOLCHAIN}" "${STABLE_TOOLCHAIN}" "${BETA_TOOLCHAIN}"; do \
        rustup component add --toolchain "${toolchain}" clippy rustfmt; \
        rustup target add --toolchain "${toolchain}" \
            thumbv7em-none-eabihf \
            thumbv6m-none-eabi \
            wasm32-unknown-unknown; \
        done \
    && rustup component add --toolchain "${STABLE_TOOLCHAIN}" llvm-tools-preview

RUN --mount=type=cache,id=deku-tools-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deku-tools-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=deku-tools-target,target=/tmp/cargo-target,sharing=locked \
    export CARGO_TARGET_DIR=/tmp/cargo-target \
    && cargo +${STABLE_TOOLCHAIN} install just --version "${JUST_VERSION}" --locked \
    && cargo +${STABLE_TOOLCHAIN} install wasm-pack --version "${WASM_PACK_VERSION}" --locked \
    && cargo +${STABLE_TOOLCHAIN} install cargo-llvm-cov --version "${CARGO_LLVM_COV_VERSION}" --locked

RUN groupadd --gid "${USER_GID}" deku \
    && useradd --uid "${USER_UID}" --gid "${USER_GID}" --create-home --shell /bin/bash deku \
    && install -d -o deku -g deku \
        /home/deku/.cargo/registry \
        /home/deku/.cargo/git \
        /workspace/target

ENV CARGO_HOME=/home/deku/.cargo \
    CARGO_TARGET_DIR=/workspace/target \
    CARGO_TERM_COLOR=always \
    DEKU_STABLE_TOOLCHAIN=${STABLE_TOOLCHAIN} \
    DEKU_MSRV_TOOLCHAIN=${MSRV_TOOLCHAIN} \
    DEKU_BETA_TOOLCHAIN=${BETA_TOOLCHAIN}

USER deku
WORKDIR /workspace
