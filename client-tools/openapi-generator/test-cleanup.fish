#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l test_root (mktemp -d)
set -g openapi_generator_test_root "$test_root"

function cleanup --on-event fish_exit
    if set -q openapi_generator_test_root
        set -l root "$openapi_generator_test_root"
        set -e openapi_generator_test_root
        if test -d "$root"
            rm -rf "$root"
        end
    end
end

set -l stub_dir "$test_root/stubs"
set -l generator_temp "$test_root/generator-temp"
set -l docker_marker "$test_root/docker-called"
set -l fish_binary (command -v fish)
mkdir -p "$stub_dir"

printf '%s\n' \
    '#!/bin/sh' \
    'mkdir -p "$OPENAPI_GENERATOR_TEST_TEMP"' \
    'printf "%s\\n" "$OPENAPI_GENERATOR_TEST_TEMP"' > "$stub_dir/mktemp"
printf '%s\n' \
    '#!/bin/sh' \
    'touch "$OPENAPI_GENERATOR_TEST_DOCKER_MARKER"' \
    'exit 1' > "$stub_dir/docker"
chmod +x "$stub_dir/mktemp" "$stub_dir/docker"

set -l inherited_path (string join : $PATH)
set -l test_path "$stub_dir:$inherited_path"

env "PATH=$test_path" \
    "OPENAPI_GENERATOR_TEST_TEMP=$generator_temp" \
    "OPENAPI_GENERATOR_TEST_DOCKER_MARKER=$docker_marker" \
    "$fish_binary" --no-config "$script_dir/generate-swift.fish" >/dev/null 2>/dev/null
set -l generator_status $status
if test $generator_status -eq 0
    echo 'Expected generator failure was not observed.' >&2
    exit 1
end

if not test -e "$docker_marker"
    echo 'Expected Docker stub was not called.' >&2
    exit 1
end

if test -e "$generator_temp"
    echo 'Temporary generation directory was not removed after failure.' >&2
    exit 1
end

echo 'failure_cleanup=passed'
