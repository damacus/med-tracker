#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l android_root (realpath "$script_dir/..")
set -l temporary_root (mktemp -d)
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'

function cleanup --on-event fish_exit
    rm -rf "$temporary_root"
end

fish --no-config "$script_dir/api-generator-input.fish" --source "$android_root/openapi.v1.yaml" --output "$temporary_root/openapi-generator-input.yaml"
or exit 1
set -l canonical_checksum (shasum -a 256 "$android_root/openapi.v1.yaml" | string split -m1 ' ')[1]
set -l derived_checksum (shasum -a 256 "$temporary_root/openapi-generator-input.yaml" | string split -m1 ' ')[1]
rg --fixed-strings --line-regexp --quiet -- "- Pinned OpenAPI SHA-256: $canonical_checksum" "$android_root/OPENAPI_PROVENANCE.md"
or exit 1
rg --fixed-strings --line-regexp --quiet -- "- Derived generator-input SHA-256: $derived_checksum" "$android_root/OPENAPI_PROVENANCE.md"
or exit 1
docker run --rm --user (id -u):(id -g) -v "$android_root:/android:ro" -v "$temporary_root:/output" $image generate -i /output/openapi-generator-input.yaml -g kotlin -c /android/openapi-generator-config.yaml --global-property apiDocs=false,modelDocs=false -o /output/release --ignore-file-override /android/openapi-generator-release.ignore
or exit 1
diff -ruN "$android_root/phone/src/main/kotlin/io/medtracker/client" "$temporary_root/release/src/main/kotlin/io/medtracker/client"
or exit 1

cd "$android_root"
shasum -a 256 -c import-manifest.sha256
