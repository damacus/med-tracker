FROM rust:1.98.1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends fish curl ca-certificates && rm -rf /var/lib/apt/lists/*
SHELL ["/usr/bin/fish", "-c"]

WORKDIR /src
COPY rust/api/Cargo.toml rust/api/Cargo.lock rust/api/
COPY rust/contract-tests/Cargo.toml rust/contract-tests/Cargo.lock rust/contract-tests/
RUN mkdir -p rust/api/src rust/contract-tests/src && echo 'pub fn placeholder() {}' > rust/api/src/lib.rs && echo 'fn main() {}' > rust/api/src/main.rs && echo 'pub fn placeholder() {}' > rust/contract-tests/src/lib.rs
RUN cargo build --locked --manifest-path rust/api/Cargo.toml; and cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --lib --no-run
COPY rust/api rust/api
COPY rust/contract-tests rust/contract-tests
RUN find rust/api/src rust/contract-tests/src rust/contract-tests/tests -type f -exec touch {} +; and cargo build --locked --manifest-path rust/api/Cargo.toml; and fish rust/contract-tests/run_medication_api_tests.fish --no-run
