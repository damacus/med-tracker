# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
FROM node:26.13.0-trixie-slim@sha256:e4db4c914bf8ad241bd24804fc25d272d89bad97e68cdc8a1414862790a02c39 AS frontend-dependencies
WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=cache,id=loco-npm,target=/root/.npm,sharing=locked \
    npm ci --ignore-scripts --no-audit --no-fund

FROM frontend-dependencies AS frontend
COPY assets/ assets/
RUN ./node_modules/.bin/tailwindcss -i assets/static/css/app.css -o assets/static/css/compiled.css --minify \
    && cp node_modules/html5-qrcode/html5-qrcode.min.js assets/static/js/html5-qrcode.min.js

FROM rust:1.99.1-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS toolchain
FROM ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c AS dependencies
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
FROM dependencies AS development
RUN apt-get update && apt-get install --yes --no-install-recommends \
    curl libasound2t64 libatk-bridge2.0-0 libatk1.0-0 libatspi2.0-0 libcups2t64 \
    libdbus-1-3 libdrm2 libgbm1 libnspr4 libnss3 libx11-6 libxcb1 libxcomposite1 \
    libxdamage1 libxext6 libxfixes3 libxkbcommon0 libxrandr2 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=frontend-dependencies /usr/local/ /usr/local/
RUN cargo install --locked cargo-nextest --version 0.9.150 \
    && cargo install --locked cargo-watch --version 8.5.3
RUN rustup component add clippy rustfmt
RUN apt-get update && apt-get install --yes --no-install-recommends \
    libcairo2 libpango-1.0-0 fonts-liberation \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_INCREMENTAL=1
COPY --from=frontend-dependencies /app/node_modules/ node_modules/
COPY package.json package-lock.json ./

FROM development AS test
COPY . .
COPY --from=frontend /app/assets/ assets/

FROM dependencies AS build
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
