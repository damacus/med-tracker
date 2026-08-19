#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l repo_root (realpath "$script_dir/../..")
set -l output "$repo_root/tmp/api-clients/swift"
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
set -l host_uid (id -u)
set -l host_gid (id -g)
set -l temporary_root (mktemp -d)
or exit 1

function cleanup --no-scope-shadowing --on-event fish_exit
    rm -rf "$temporary_root"
end

function generate_swift_package --no-scope-shadowing
    set -l destination $argv[1]

    mkdir -p "$destination"
    docker run --rm --user "$host_uid:$host_gid" \
        -v "$repo_root:/local:ro" \
        -v "$temporary_root:/output" \
        $image generate \
        -i /local/docs/api/openapi.v1.yaml \
        -g swift6 \
        -c /local/client-tools/openapi-generator/swift-config.yaml \
        --global-property apiDocs=false,modelDocs=false \
        -o "/output/"(basename "$destination")
end

generate_swift_package "$temporary_root/generated"
or exit 1

if test "$argv[1]" = determinism
    generate_swift_package "$temporary_root/repeated"
    or exit 1
    diff -ruN "$temporary_root/generated" "$temporary_root/repeated"
    or exit 1
end

rm -rf "$output"
mkdir -p (dirname "$output")
mv "$temporary_root/generated" "$output"
