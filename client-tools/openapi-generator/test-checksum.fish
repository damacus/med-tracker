#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l test_root (mktemp -d)
set -g openapi_generator_checksum_test_root "$test_root"

function cleanup --on-event fish_exit
    if set -q openapi_generator_checksum_test_root
        set -l root "$openapi_generator_checksum_test_root"
        set -e openapi_generator_checksum_test_root
        if test -d "$root"
            rm -rf "$root"
        end
    end
end

set -l stub_dir "$test_root/stubs"
set -l fish_binary (command -v fish)
set -l contract "$script_dir/checksum.fish"
set -l expected_digest (string repeat -n 64 a)
mkdir -p "$stub_dir"

printf '%s\n' \
    '#!/bin/sh' \
    'printf "%s  %s\\n" "$OPENAPI_TEST_DIGEST" "$2"' > "$stub_dir/shasum"
chmod +x "$stub_dir/shasum"

set -lx OPENAPI_TEST_DIGEST "$expected_digest"
set -l checksum_command "source (string escape \"$script_dir/checksum.fish\"); openapi_contract_checksum (string escape \"$contract\")"
set -l checksum (env PATH="$stub_dir" "$fish_binary" --no-config -c "$checksum_command")
if test $status -ne 0; or test "$checksum" != "$expected_digest"
    echo 'The shasum fallback did not produce the expected digest.' >&2
    exit 1
end

set -lx OPENAPI_TEST_DIGEST 'ABC123'
set checksum (env PATH="$stub_dir" "$fish_binary" --no-config -c "$checksum_command")
if test $status -eq 0
    echo 'The checksum helper accepted a non-canonical digest.' >&2
    exit 1
end

rm "$stub_dir/shasum"
set checksum (env PATH="$stub_dir" "$fish_binary" --no-config -c "$checksum_command")
if test $status -eq 0
    echo 'The checksum helper accepted a missing checksum tool.' >&2
    exit 1
end

echo 'checksum_selection=passed'
