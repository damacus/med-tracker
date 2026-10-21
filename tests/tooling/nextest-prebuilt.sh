#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/tools" "$fixture/bin"
export NEXTEST_VERSION=0.9.149
export GITHUB_PATH="$fixture/github-path"
export NEXTEST_FIXTURE="$fixture"
cat > "$fixture/nextest" <<'SH'
#!/usr/bin/env bash
touch "$NEXTEST_FIXTURE/executed"
echo "cargo-nextest ${NEXTEST_FIXTURE_VERSION:-0.9.149} (fixture)"
SH
chmod +x "$fixture/nextest"
export NEXTEST_SHA256
NEXTEST_SHA256=$(sha256sum "$fixture/nextest" | cut -d ' ' -f 1)
cat > "$fixture/tools/cargo-binstall" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "$NEXTEST_FIXTURE/install-args"
[[ "$*" == "cargo-nextest@0.9.149 --strategies crate-meta-data --no-confirm --locked --no-track --force --install-path $NEXTEST_FIXTURE/bin" ]]
[[ "${NEXTEST_FIXTURE_UNAVAILABLE:-false}" != true ]] || exit 42
cp "$NEXTEST_FIXTURE/nextest" "$NEXTEST_FIXTURE/bin/cargo-nextest"
SH
cat > "$fixture/tools/cargo" <<'SH'
#!/usr/bin/env bash
touch "$NEXTEST_FIXTURE/source-compile"
exit 99
SH
chmod +x "$fixture/tools/"*
export PATH="$fixture/tools:$PATH"
installer="$root/.github/actions/nextest/install.sh"

expect_failure() {
  local expected_message=$1
  shift
  if "$@" > "$fixture/expected-failure.log" 2>&1; then
    cat "$fixture/expected-failure.log" >&2
    echo "Unexpected success; expected: $expected_message" >&2
    exit 1
  fi
  if [[ "$(< "$fixture/expected-failure.log")" != *"$expected_message"* ]]; then
    cat "$fixture/expected-failure.log" >&2
    echo "Missing expected error: $expected_message" >&2
    exit 1
  fi
}

NEXTEST_CACHE_HIT=false bash "$installer" "$fixture/bin/cargo-nextest"
test -f "$fixture/executed"
test "$(cat "$GITHUB_PATH")" = "$fixture/bin"
rm "$fixture/install-args" "$fixture/executed" "$GITHUB_PATH"
NEXTEST_CACHE_HIT=true bash "$installer" "$fixture/bin/cargo-nextest"
test -f "$fixture/executed"
test ! -f "$fixture/install-args"

for cache_hit in true false; do
  rm -f "$fixture/executed" "$GITHUB_PATH"
  expect_failure 'computed checksum did NOT match' env \
    NEXTEST_SHA256="$(printf '0%.0s' {1..64})" NEXTEST_CACHE_HIT=$cache_hit \
    bash "$installer" "$fixture/bin/cargo-nextest"
  test ! -f "$fixture/executed"
  test ! -f "$GITHUB_PATH"
done

rm -f "$fixture/bin/cargo-nextest" "$fixture/install-args"
expect_failure 'Pinned nextest prebuilt installation failed; source compilation is disabled' env \
  NEXTEST_FIXTURE_UNAVAILABLE=true NEXTEST_CACHE_HIT=false \
  bash "$installer" "$fixture/bin/cargo-nextest"
test ! -f "$fixture/source-compile"
test ! -f "$fixture/executed"
test ! -f "$GITHUB_PATH"

cp "$fixture/nextest" "$fixture/bin/cargo-nextest"
expect_failure 'Unexpected nextest version: cargo-nextest 0.9.148' env \
  NEXTEST_FIXTURE_VERSION=0.9.148 NEXTEST_CACHE_HIT=true \
  bash "$installer" "$fixture/bin/cargo-nextest"
test ! -f "$GITHUB_PATH"
echo 'Prebuilt installation, exact cache hit, integrity failures, unavailable asset and version checks passed'
