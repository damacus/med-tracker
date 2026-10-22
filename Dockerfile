# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
FROM node:24-trixie-slim@sha256:43b9ba1142b8dcf33fa8f922ca1be85714adc444e0586c7cf7726daaeabc1bea AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts --no-audit --no-fund
COPY assets/ assets/
RUN ./node_modules/.bin/tailwindcss -i assets/static/css/app.css -o assets/static/css/compiled.css --minify \
    && cp node_modules/html5-qrcode/html5-qrcode.min.js assets/static/js/html5-qrcode.min.js

FROM rust:1.99.1-slim-trixie@sha256:aab42391a986424e22d6a3ec14f37bc6dd3dacff4d457ca7cccd781e02297cc6 AS toolchain
FROM ubuntu:26.04@sha256:c098726dd9b9733f3e885f8de0ef375206a311b0a317eadc40d347b1115fb944 AS build
ARG CARGO_BUILD_JOBS=2
ARG TARGETARCH
RUN apt-get update && apt-get install --yes --no-install-recommends \
    build-essential ca-certificates clang libclang-dev libicu-dev libssl-dev pkg-config tzdata \
    && rm -rf /var/lib/apt/lists/*
COPY --from=toolchain /usr/local/cargo /usr/local/cargo
COPY --from=toolchain /usr/local/rustup /usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_ICU_MAJOR_VERSION_NUMBER=78 \
    CARGO_INCREMENTAL=0
WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY migration/ migration/
COPY src/ src/
COPY assets/ assets/
COPY rails/config/ai_medication_sources.yml rails/config/medication_review_labels.yml rails/config/
RUN --mount=type=cache,id=loco-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=loco-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=loco-target-${TARGETARCH},target=/app/target,sharing=locked \
    cargo build --locked --release --bin med-tracker \
    && install -Dm755 target/release/med-tracker /rootfs/app/med-tracker
RUN ldd /rootfs/app/med-tracker > /tmp/dependencies \
    && ! grep -q 'not found' /tmp/dependencies \
    && awk '/=> \// {print $3} /^[[:space:]]*\// {print $1}' /tmp/dependencies \
       | xargs -r -I '{}' cp --parents -L '{}' /rootfs \
    && mkdir -p /rootfs/etc/ssl/certs /rootfs/usr/share /rootfs/tmp \
    && chmod 1777 /rootfs/tmp \
    && cp /etc/ssl/certs/ca-certificates.crt /rootfs/etc/ssl/certs/ \
    && cp -a /usr/share/zoneinfo /rootfs/usr/share/

FROM scratch AS runtime
COPY --from=build /rootfs/ /
WORKDIR /app
COPY config/production.yaml config/production.yaml
COPY --from=frontend /app/assets/ assets/
ENV BINDING=0.0.0.0 \
    PORT=5150 \
    TZ=UTC \
    SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
USER 65532:65532
EXPOSE 5150
ENTRYPOINT ["/app/med-tracker"]
CMD ["start", "--environment", "production"]
