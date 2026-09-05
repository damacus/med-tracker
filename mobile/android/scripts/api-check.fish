#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l android_root (realpath "$script_dir/..")
set -l temporary_root (mktemp -d)
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'

function cleanup --on-event fish_exit
    rm -rf "$temporary_root"
end

docker run --rm --user (id -u):(id -g) -v "$android_root:/android:ro" -v "$temporary_root:/output" $image generate -i /android/openapi.v1.yaml -g kotlin -c /android/openapi-generator-config.yaml --global-property apiDocs=false,modelDocs=false -o /output/client
or exit 1

diff -ruN "$android_root/phone/src/main/kotlin/io/medtracker/client" "$temporary_root/client/src/main/kotlin/io/medtracker/client"
or exit 1

cd "$android_root"
shasum -a 256 -c import-manifest.sha256
