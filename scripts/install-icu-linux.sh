#!/usr/bin/env bash
set -euo pipefail

if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
  echo 'ICU 78.3 binary installation supports Linux x86_64 only' >&2
  exit 1
fi

archive="${RUNNER_TEMP:-/tmp}/icu4c-78.3-Ubuntu22.04-x64.tgz"
prefix=/opt/medtracker-icu
url=https://github.com/unicode-org/icu/releases/download/release-78.3/icu4c-78.3-Ubuntu22.04-x64.tgz
checksum=9c8abe58a20cf82ea22341d8125272094f7fca2414173cbc2d8742feb84566a2

curl --fail --location --retry 3 --output "$archive" "$url"
echo "$checksum  $archive" | sha256sum --check --status
sudo mkdir -p "$prefix"
sudo tar -xzf "$archive" -C "$prefix" --strip-components=3
for library in icuuc icui18n icudata; do
  test -f "$prefix/lib/lib${library}.so.78"
done
echo "Installed ICU 78.3 in $prefix"
