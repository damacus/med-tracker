#!/usr/bin/env bash
set -euo pipefail

binary=$1
if [[ "${NEXTEST_CACHE_HIT:-false}" != true ]]; then
  if ! cargo-binstall "cargo-nextest@$NEXTEST_VERSION" --strategies crate-meta-data \
    --no-confirm --locked --no-track --force --install-path "$(dirname "$binary")"; then
    echo '::error::Pinned nextest prebuilt installation failed; source compilation is disabled' >&2
    exit 1
  fi
fi

printf '%s  %s\n' "$NEXTEST_SHA256" "$binary" | sha256sum --check --strict
installed_version=$("$binary" --version)
if [[ "$installed_version" != "cargo-nextest $NEXTEST_VERSION ("* ]]; then
  echo "::error::Unexpected nextest version: $installed_version" >&2
  exit 1
fi
echo "$installed_version (verified SHA-256: $NEXTEST_SHA256)"
dirname "$binary" >> "$GITHUB_PATH"
