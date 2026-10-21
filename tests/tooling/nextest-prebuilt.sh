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

NEXTEST_CACHE_HIT=false bash "$installer" "$fixture/bin/cargo-nextest"
test -f "$fixture/executed"
test "$(cat "$GITHUB_PATH")" = "$fixture/bin"
rm "$fixture/install-args" "$fixture/executed" "$GITHUB_PATH"
NEXTEST_CACHE_HIT=true bash "$installer" "$fixture/bin/cargo-nextest"
test -f "$fixture/executed"
test ! -f "$fixture/install-args"

for cache_hit in true false; do
  rm -f "$fixture/executed" "$GITHUB_PATH"
  if NEXTEST_SHA256=$(printf '0%.0s' {1..64}) NEXTEST_CACHE_HIT=$cache_hit bash "$installer" "$fixture/bin/cargo-nextest"; then
    echo "Accepted corrupt binary with cache-hit=$cache_hit" >&2
    exit 1
  fi
  test ! -f "$fixture/executed"
  test ! -f "$GITHUB_PATH"
done

rm -f "$fixture/bin/cargo-nextest" "$fixture/install-args"
if NEXTEST_FIXTURE_UNAVAILABLE=true NEXTEST_CACHE_HIT=false bash "$installer" "$fixture/bin/cargo-nextest"; then
  echo 'Accepted unavailable prebuilt asset' >&2
  exit 1
fi
test ! -f "$fixture/source-compile"
test ! -f "$fixture/executed"
test ! -f "$GITHUB_PATH"

cp "$fixture/nextest" "$fixture/bin/cargo-nextest"
if NEXTEST_FIXTURE_VERSION=0.9.148 NEXTEST_CACHE_HIT=true bash "$installer" "$fixture/bin/cargo-nextest"; then
  echo 'Accepted incorrect version' >&2
  exit 1
fi
test ! -f "$GITHUB_PATH"
echo 'Prebuilt installation, exact cache hit, integrity failures, unavailable asset and version checks passed'
