#!/usr/bin/env bash
set -euo pipefail

if [[ $(uname -s) != Linux ]]; then
  echo 'Linux ICU runtime linkage check requires Linux' >&2
  exit 1
fi

checked=0
for binary in target/debug/deps/*; do
  [[ -f "$binary" && -x "$binary" ]] || continue
  linkage=$(ldd "$binary" 2>/dev/null) || continue
  if [[ "$linkage" != *libicuuc.so.78* ]]; then
    continue
  fi
  for library in icuuc icui18n icudata; do
    if ! grep -Fq "lib${library}.so.78 => /opt/medtracker-icu/lib/lib${library}.so.78" <<< "$linkage"; then
      echo "Unexpected ICU linkage in $binary:" >&2
      echo "$linkage" >&2
      exit 1
    fi
  done
  echo "ICU 78.3 runtime linkage verified: $binary"
  checked=$((checked + 1))
done

if (( checked == 0 )); then
  echo 'No ICU-linked test executable was found' >&2
  exit 1
fi
