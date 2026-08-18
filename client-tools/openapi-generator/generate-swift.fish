#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l repo_root (realpath "$script_dir/../..")
set -l contract "$repo_root/docs/api/openapi.v1.yaml"
set -l output "$repo_root/client-tools/generated/swift"
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
source "$script_dir/checksum.fish"
source "$script_dir/docker-runtime.fish"

function cleanup --on-event fish_exit
    if set -q med_tracker_openapi_temporary_root
        set -l root "$med_tracker_openapi_temporary_root"
        set -e med_tracker_openapi_temporary_root
        if test -d "$root"
            rm -rf "$root"
        end
    end
end

set -l temporary_root (mktemp -d)
if test $status -ne 0
    exit 1
end
set -g med_tracker_openapi_temporary_root "$temporary_root"

function generate_swift_package --no-scope-shadowing
    set -l output_name $argv[1]
    set -l temporary_output "$temporary_root/$output_name"

    mkdir -p "$temporary_output"
    openapi_generator_docker_run --rm \
        -v "$repo_root:/local" \
        -v "$temporary_root:/tmp/openapi" \
        $image generate \
        -i /local/docs/api/openapi.v1.yaml \
        -g swift6 \
        -c /local/client-tools/openapi-generator/swift-config.yaml \
        -o "/tmp/openapi/$output_name"
    return $status
end

function add_contract_checksum --no-scope-shadowing
    set -l package_root $argv[1]
    set -l contract_sha256 (openapi_contract_checksum "$contract")
    if test $status -ne 0
        return 1
    end
    if test (count $contract_sha256) -ne 1
        echo 'Expected one contract checksum.' >&2
        return 1
    end
    printf '%s\n' "$contract_sha256" > "$package_root/OPENAPI_SHA256"
end

if not generate_swift_package output
    exit 1
end
if not add_contract_checksum "$temporary_root/output"
    exit 1
end

if test "$argv[1]" = determinism
    if not generate_swift_package repeat
        exit 1
    end
    if not add_contract_checksum "$temporary_root/repeat"
        exit 1
    end
    diff -ruN "$temporary_root/output" "$temporary_root/repeat"
    exit $status
end

if test "$argv[1]" = verify
    if not test -d "$output"
        echo "Missing generated Swift output: $output" >&2
        exit 1
    end
    diff -ruN --exclude OPENAPI_SHA256 --exclude .build --exclude build --exclude .swiftpm "$output" "$temporary_root/output"
    if test $status -ne 0
        exit 1
    end
    diff -u "$output/OPENAPI_SHA256" "$temporary_root/output/OPENAPI_SHA256"
    exit $status
end

set -l staged_output "$temporary_root/staged"
mv "$temporary_root/output" "$staged_output"
rm -rf "$output"
mkdir -p (dirname "$output")
mv "$staged_output" "$output"
