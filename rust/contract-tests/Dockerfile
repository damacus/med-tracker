FROM rust:1.99.0-bookworm AS base

RUN apt-get update && apt-get install -y --no-install-recommends fish curl ca-certificates file musl-tools perl poppler-utils && rm -rf /var/lib/apt/lists/*
SHELL ["/usr/bin/fish", "-c"]
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

WORKDIR /src
COPY rust/api/Cargo.toml rust/api/Cargo.lock rust/api/
COPY rust/vendor rust/vendor
COPY rust/web/Cargo.toml rust/web/Cargo.lock rust/web/
COPY rust/ui-preview/Cargo.toml rust/ui-preview/build.rs rust/ui-preview/
COPY rust/contract-tests/Cargo.toml rust/contract-tests/Cargo.lock rust/contract-tests/
RUN mkdir -p rust/api/src rust/web/src rust/ui-preview/src rust/contract-tests/src && echo 'pub fn placeholder() {}' > rust/api/src/lib.rs && echo 'fn main() {}' > rust/api/src/main.rs && echo 'pub fn placeholder() {}' > rust/web/src/lib.rs && echo 'pub fn placeholder() {}' > rust/ui-preview/src/lib.rs && echo 'pub fn placeholder() {}' > rust/contract-tests/src/lib.rs
FROM base AS dependencies
RUN cargo fetch --locked --manifest-path rust/api/Cargo.toml; and cargo fetch --locked --manifest-path rust/contract-tests/Cargo.toml
RUN cargo build --locked --manifest-path rust/api/Cargo.toml; and cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --lib --no-run
FROM dependencies AS runner
COPY rust/api rust/api
COPY rust/web/src rust/web/src
COPY rust/ui-preview/src rust/ui-preview/src
COPY rust/ui-preview/public rust/ui-preview/public
COPY config/ai_medication_sources.yml config/ai_medication_sources.yml
COPY config/nhs_dmd_curated_products.yml config/nhs_dmd_curated_products.yml
COPY config/locales config/locales
COPY vendor/fonts vendor/fonts
RUN find rust/api/src rust/web/src rust/ui-preview/src -type f -exec touch {} +; and cargo build --locked --manifest-path rust/api/Cargo.toml
COPY rust/contract-tests rust/contract-tests
RUN find rust/contract-tests/src rust/contract-tests/tests -type f -exec touch {} +; and cargo test --locked --manifest-path rust/contract-tests/Cargo.toml --lib --no-run

FROM base AS release-dependencies
ARG RUST_TARGET=x86_64-unknown-linux-musl
RUN cargo build --locked --release --config 'profile.release.strip="symbols"' --target $RUST_TARGET --manifest-path rust/api/Cargo.toml

FROM release-dependencies AS release-builder
ARG RUST_TARGET=x86_64-unknown-linux-musl
COPY rust/api rust/api
COPY rust/web/src rust/web/src
COPY rust/ui-preview/src rust/ui-preview/src
COPY rust/ui-preview/public rust/ui-preview/public
COPY config/ai_medication_sources.yml config/ai_medication_sources.yml
COPY config/nhs_dmd_curated_products.yml config/nhs_dmd_curated_products.yml
COPY config/locales config/locales
COPY vendor/fonts vendor/fonts
RUN find rust/api/src rust/web/src rust/ui-preview/src -type f -exec touch {} +; and cargo build --locked --release --config 'profile.release.strip="symbols"' --target $RUST_TARGET --manifest-path rust/api/Cargo.toml
RUN mkdir -p /runtime-root/tmp; and chmod 1777 /runtime-root/tmp
RUN file rust/api/target/$RUST_TARGET/release/medtracker-api; and file rust/api/target/$RUST_TARGET/release/medtracker-api | string match -rq 'statically linked'

FROM scratch AS runtime
ARG RUST_TARGET=x86_64-unknown-linux-musl
COPY --from=release-builder /src/rust/api/target/$RUST_TARGET/release/medtracker-api /app/medtracker-api
COPY --from=release-builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=release-builder /src/vendor/fonts/NotoSans-Regular.ttf /src/vendor/fonts/NotoSans-Regular.ttf
COPY --from=release-builder /runtime-root/tmp /tmp
WORKDIR /src
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
USER 65532:65532
ENTRYPOINT ["/app/medtracker-api"]
