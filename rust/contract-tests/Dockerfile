FROM rust:1.99.0-bookworm AS dependencies

RUN apt-get update && apt-get install -y --no-install-recommends fish curl ca-certificates poppler-utils && rm -rf /var/lib/apt/lists/*
SHELL ["/usr/bin/fish", "-c"]

WORKDIR /src
COPY rust/api/Cargo.toml rust/api/Cargo.lock rust/api/
COPY rust/web/Cargo.toml rust/web/Cargo.lock rust/web/
COPY rust/ui-preview/Cargo.toml rust/ui-preview/build.rs rust/ui-preview/
COPY rust/contract-tests/Cargo.toml rust/contract-tests/Cargo.lock rust/contract-tests/
RUN mkdir -p rust/api/src rust/web/src rust/ui-preview/src rust/contract-tests/src && echo 'pub fn placeholder() {}' > rust/api/src/lib.rs && echo 'fn main() {}' > rust/api/src/main.rs && echo 'pub fn placeholder() {}' > rust/web/src/lib.rs && echo 'pub fn placeholder() {}' > rust/ui-preview/src/lib.rs && echo 'pub fn placeholder() {}' > rust/contract-tests/src/lib.rs
RUN cargo build --locked --manifest-path rust/api/Cargo.toml; and cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --lib --no-run
FROM dependencies AS runner
COPY rust/api rust/api
COPY rust/web/src rust/web/src
COPY rust/ui-preview/src rust/ui-preview/src
COPY rust/ui-preview/public rust/ui-preview/public
COPY config/ai_medication_sources.yml config/ai_medication_sources.yml
COPY config/nhs_dmd_curated_products.yml config/nhs_dmd_curated_products.yml
COPY vendor/fonts vendor/fonts
RUN find rust/api/src rust/web/src rust/ui-preview/src -type f -exec touch {} +; and cargo build --locked --manifest-path rust/api/Cargo.toml
COPY rust/contract-tests rust/contract-tests
RUN find rust/contract-tests/src rust/contract-tests/tests -type f -exec touch {} +; and cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --lib --no-run
