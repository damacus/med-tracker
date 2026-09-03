#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l android_root (realpath "$script_dir/..")
set -l repo_root (realpath "$android_root/../..")
set -l temporary_root (mktemp -d)
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
set -l output "$android_root/phone/src/main/kotlin/io/medtracker/client"
set -l source_revision (git -C "$repo_root" log -1 --format=%H -- docs/api/openapi.v1.yaml)
set -l source_checksum (shasum -a 256 "$repo_root/docs/api/openapi.v1.yaml" | string split -m1 ' ')[1]

function cleanup --on-event fish_exit
    rm -rf "$temporary_root"
end

cp -p "$repo_root/docs/api/openapi.v1.yaml" "$android_root/openapi.v1.yaml"
docker run --rm --user (id -u):(id -g) -v "$android_root:/android:ro" -v "$temporary_root:/output" $image generate -i /android/openapi.v1.yaml -g kotlin -c /android/openapi-generator-config.yaml --global-property apiDocs=false,modelDocs=false -o /output/client
or exit 1

rm -rf "$output"
mkdir -p (dirname "$output")
cp -R "$temporary_root/client/src/main/kotlin/io/medtracker/client" "$output"
printf '%s\n' '# Android OpenAPI generated-client provenance' '' '- Pinned OpenAPI source: docs/api/openapi.v1.yaml' "- Source revision: $source_revision" "- Pinned OpenAPI SHA-256: $source_checksum" "- Generator image: $image" '- Generator: OpenAPI Generator 7.20.0' '- Generator configuration: mobile/android/openapi-generator-config.yaml (jvm-okhttp4 with Moshi)' '- Generated Kotlin artifacts: mobile/android/phone/src/main/kotlin/io/medtracker/client' > "$android_root/OPENAPI_PROVENANCE.md"

begin
    for path in openapi.v1.yaml openapi-generator-config.yaml (find phone/src/main/kotlin/io/medtracker/client -type f | sort)
        shasum -a 256 "$path"
    end
end > "$android_root/import-manifest.sha256"
