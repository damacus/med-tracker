#!/usr/bin/env bash
set -euo pipefail

if [[ $(uname -s) != Linux ]]; then
  echo 'Linux ICU runtime linkage check requires Linux' >&2
  exit 1
fi

icu_lib=${RUST_ICU_LINK_SEARCH_DIR:?Run this check through scripts/with-icu.fish}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
probe_dir=$(mktemp -d "${RUNNER_TEMP:-/tmp}/medtracker-icu-runtime.XXXXXX")
trap 'rm -rf -- "$probe_dir"' EXIT

check_linkage() {
  local binary=$1 linkage=$2 mode=${3:-all}
  for library in icuuc icui18n icudata; do
    if [[ "$mode" == referenced ]] && ! grep -Eq "lib${library}\\.so" <<< "$linkage"; then
      continue
    fi
    if ! grep -Fq "lib${library}.so.78 => $icu_lib/lib${library}.so.78 " <<< "$linkage"; then
      echo "Unexpected ICU linkage in $binary:" >&2
      echo "$linkage" >&2
      exit 1
    fi
  done
  echo "ICU 78.3 runtime linkage verified: $binary"
}

cc -Wall -Wextra -Werror -I"$icu_lib/../include" "$script_dir/icu-runtime-probe.c" \
  -L"$icu_lib" -Wl,--no-as-needed -licui18n -licuuc -licudata -o "$probe_dir/probe"
linkage=$(ldd "$probe_dir/probe")
echo "$linkage"
check_linkage "$probe_dir/probe" "$linkage"
"$probe_dir/probe"

metadata=$(cargo metadata --locked --offline --no-deps --format-version 1)
directories=$(jq -er '[.target_directory, .build_directory] | map(select(type == "string")) | unique[]' <<< "$metadata")
checked=0
while IFS= read -r directory; do
  echo "Inspecting Cargo test artifacts: $directory/debug/deps"
  for binary in "$directory"/debug/deps/*; do
    [[ -f "$binary" && -x "$binary" ]] || continue
    linkage=$(ldd "$binary" 2>/dev/null) || continue
    echo "Runtime dependencies: $binary"
    echo "$linkage"
    if grep -Eq 'libicu(uc|i18n|data)\.so' <<< "$linkage"; then
      check_linkage "$binary" "$linkage" referenced
      checked=$((checked + 1))
    fi
  done
done <<< "$directories"
echo "ICU-linked Cargo test executables verified: $checked; native runtime probe passed"
