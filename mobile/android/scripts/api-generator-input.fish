#!/usr/bin/env fish

argparse 'source=' 'output=' -- $argv
or exit 64
if not set -q _flag_source; or not set -q _flag_output
    echo 'Missing generator-input --source or --output' >&2
    exit 64
end
if not test -f "$_flag_source"; or test (path resolve "$_flag_source") = (path resolve "$_flag_output")
    echo 'Generator input requires an existing source and a separate output' >&2
    exit 1
end
set -l document (string collect <"$_flag_source")
or exit 1
printf '%s\n' "$document" | cmp -s "$_flag_source" -
or begin
    echo 'Canonical generator input must have exactly one final newline' >&2
    exit 1
end
if test (string match --all --regex -- '(?m)^    PortableImportUnprocessableResponse:$' "$document" | count) -ne 1
    echo 'Expected exactly one canonical portable-import response schema' >&2
    exit 1
end
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
set -l derived (string replace -- "$canonical" "$flat" "$document" | string collect)
if test "$derived" = "$document"
    echo 'Canonical portable-import response drifted; review the generator-input adaptation' >&2
    exit 1
end
set -l restored (string replace -- "$flat" "$canonical" "$derived" | string collect)
if test "$restored" != "$document"
    echo 'Generator-input adaptation did not preserve the remaining canonical document' >&2
    exit 1
end
printf '%s\n' "$derived" >"$_flag_output"
