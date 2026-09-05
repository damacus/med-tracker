#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l android_root (realpath "$script_dir/..")
set -l temporary_root (mktemp -d)
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'

function cleanup --on-event fish_exit
    rm -rf "$temporary_root"
end

set -l release_operation_ids (
    string match --regex --groups-only '^\s+operationId:\s+(\S+)\s*$' < "$android_root/openapi.v1.yaml" |
        string match --invert createLoginSession
)
set -l release_filter "FILTER=operationId:"(string join '|' $release_operation_ids)

docker run --rm --user (id -u):(id -g) -v "$android_root:/android:ro" -v "$temporary_root:/output" $image generate -i /android/openapi.v1.yaml -g kotlin -c /android/openapi-generator-config.yaml --global-property apiDocs=false,modelDocs=false -o /output/release --openapi-normalizer "$release_filter" --ignore-file-override /android/openapi-generator-release.ignore
or exit 1
rm -f "$temporary_root/release/src/main/kotlin/io/medtracker/client/models/AuthLoginRequest.kt"
docker run --rm --user (id -u):(id -g) -v "$android_root:/android:ro" -v "$temporary_root:/output" $image generate -i /android/openapi.v1.yaml -g kotlin -c /android/openapi-generator-password-config.yaml --global-property apiDocs=false,modelDocs=false -o /output/password --openapi-normalizer 'FILTER=operationId:createLoginSession'
or exit 1

mkdir -p "$temporary_root/nonRelease/io/medtracker/password/client/apis" "$temporary_root/nonRelease/io/medtracker/client/models"
sed -E 's/[[:blank:]]+$//' "$temporary_root/password/src/main/kotlin/io/medtracker/password/client/apis/AuthenticationApi.kt" > "$temporary_root/nonRelease/io/medtracker/password/client/apis/AuthenticationApi.kt"
cp -p "$temporary_root/password/src/main/kotlin/io/medtracker/client/models/AuthLoginRequest.kt" "$temporary_root/nonRelease/io/medtracker/client/models/AuthLoginRequest.kt"

diff -ruN "$android_root/phone/src/main/kotlin/io/medtracker/client" "$temporary_root/release/src/main/kotlin/io/medtracker/client"
or exit 1
diff -ruN "$android_root/phone/src/nonRelease/kotlin" "$temporary_root/nonRelease"
or exit 1

cd "$android_root"
shasum -a 256 -c import-manifest.sha256
