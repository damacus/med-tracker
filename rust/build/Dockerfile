FROM node:24.18.0-bookworm-slim AS node

FROM rust:1.99.0-bookworm

COPY --from=node /usr/local/ /usr/local/

RUN apt-get update && apt-get install -y --no-install-recommends build-essential ca-certificates pkg-config && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy rustfmt && rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.127 --locked --root /opt/rust-tools

ENV PATH="/opt/rust-tools/bin:${PATH}"
WORKDIR /workspace
