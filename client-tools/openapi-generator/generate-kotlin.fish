#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l repo_root (realpath "$script_dir/../..")
set -l contract "$repo_root/docs/api/openapi.v1.yaml"
set -l output "$repo_root/client-tools/generated/kotlin"
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
set -l temporary_root (mktemp -d)
set -l temporary_output "$temporary_root/output"

function cleanup --on-event fish_exit
    if test -n "$temporary_root"; and test -d "$temporary_root"
        rm -rf "$temporary_root"
    end
end

mkdir -p "$temporary_output"
docker run --rm \
    -v "$repo_root:/local" \
    -v "$temporary_root:/tmp/openapi" \
    $image generate \
    -i /local/docs/api/openapi.v1.yaml \
    -g kotlin \
    -c /local/client-tools/openapi-generator/kotlin-config.yaml \
    -o /tmp/openapi/output
if test $status -ne 0
    exit 1
end

sha256sum "$contract" | string split -m1 ' ' | read -l contract_sha256
printf '%s\n' "$contract_sha256" > "$temporary_output/OPENAPI_SHA256"

if test "$argv[1]" = verify
    if not test -d "$output"
        echo "Missing generated Kotlin output: $output" >&2
        exit 1
    end
    diff -ruN --exclude OPENAPI_SHA256 "$output" "$temporary_output"
    if test $status -ne 0
        exit 1
    end
    diff -u "$output/OPENAPI_SHA256" "$temporary_output/OPENAPI_SHA256"
    exit $status
end

set -l staged_output "$temporary_root/staged"
mv "$temporary_output" "$staged_output"
rm -rf "$output"
mkdir -p (dirname "$output")
mv "$staged_output" "$output"
