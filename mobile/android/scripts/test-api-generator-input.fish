#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l android_root (realpath "$script_dir/..")
set -l adapter "$script_dir/api-generator-input.fish"
set -l temporary_root (mktemp -d)

function cleanup --on-event fish_exit
    rm -rf "$temporary_root"
end

if not test -f "$adapter"
    echo 'Missing Android generator-input adapter' >&2
    exit 1
end

fish --no-config "$adapter" --source "$android_root/openapi.v1.yaml" --output "$temporary_root/first.yaml"
or exit 1
fish --no-config "$adapter" --source "$android_root/openapi.v1.yaml" --output "$temporary_root/second.yaml"
or exit 1
cmp -s "$temporary_root/first.yaml" "$temporary_root/second.yaml"
or exit 1

set -l canonical '    PortableImportUnprocessableResponse:
      oneOf:
        - $ref: "#/components/schemas/PortableImportResultResponse"
        - $ref: "#/components/schemas/ErrorEnvelope"
    PortableImportResult:'
set -l flat '    PortableImportUnprocessableResponse:
      type: object
      additionalProperties: false
      properties:
        data:
          $ref: "#/components/schemas/PortableImportResult"
        error:
          $ref: "#/components/schemas/Error"
    PortableImportResult:'
set -l derived (string collect <"$temporary_root/first.yaml")
set -l restored (string replace -- "$flat" "$canonical" "$derived" | string collect)
printf '%s\n' "$restored" >"$temporary_root/restored.yaml"
cmp -s "$android_root/openapi.v1.yaml" "$temporary_root/restored.yaml"
or begin
    echo 'Generator-input adaptation changed unrelated canonical bytes' >&2
    exit 1
end

set -l original (string collect <"$android_root/openapi.v1.yaml")
printf '%s\n%s\n' "$original" "$canonical" >"$temporary_root/duplicate.yaml"
if fish --no-config "$adapter" --source "$temporary_root/duplicate.yaml" --output "$temporary_root/rejected.yaml" >/dev/null 2>&1
    echo 'Duplicate target schema unexpectedly passed' >&2
    exit 1
end
set -l changed (string replace -- '    PortableImportUnprocessableResponse:' '    UnexpectedPortableResponse:' "$original" | string collect)
printf '%s\n' "$changed" >"$temporary_root/missing.yaml"
if fish --no-config "$adapter" --source "$temporary_root/missing.yaml" --output "$temporary_root/rejected.yaml" >/dev/null 2>&1
    echo 'Missing target schema unexpectedly passed' >&2
    exit 1
end
set -l drifted (string replace -- "$canonical" (string replace '      oneOf:' '      anyOf:' "$canonical" | string collect) "$original" | string collect)
printf '%s\n' "$drifted" >"$temporary_root/drifted.yaml"
if fish --no-config "$adapter" --source "$temporary_root/drifted.yaml" --output "$temporary_root/rejected.yaml" >/dev/null 2>&1
    echo 'Changed target schema unexpectedly passed' >&2
    exit 1
end

printf '%s\n' "$original" '    UnrelatedUnion:
      oneOf:
        - type: string
        - type: integer' >"$temporary_root/unrelated.yaml"
fish --no-config "$adapter" --source "$temporary_root/unrelated.yaml" --output "$temporary_root/unrelated-derived.yaml"
or exit 1
set -l unrelated (string collect <"$temporary_root/unrelated-derived.yaml")
set -l unrelated_restored (string replace -- "$flat" "$canonical" "$unrelated" | string collect)
printf '%s\n' "$unrelated_restored" >"$temporary_root/unrelated-restored.yaml"
cmp -s "$temporary_root/unrelated.yaml" "$temporary_root/unrelated-restored.yaml"
or exit 1

echo 'Android generator-input integrity and drift rejection tests passed'
