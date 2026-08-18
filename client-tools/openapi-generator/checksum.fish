function openapi_contract_checksum
    if test (count $argv) -ne 1
        echo 'Expected one OpenAPI contract path.' >&2
        return 2
    end

    set -l contract $argv[1]
    set -l checksum_line
    set -l checksum_status 1

    if type -q sha256sum
        set checksum_line (sha256sum "$contract")
        set checksum_status $status
    else if type -q shasum
        set checksum_line (shasum -a 256 "$contract")
        set checksum_status $status
    else
        echo 'No SHA-256 checksum tool found; install sha256sum or shasum.' >&2
        return 1
    end

    if test $checksum_status -ne 0
        echo "Unable to checksum OpenAPI contract: $contract" >&2
        return $checksum_status
    end

    set -l checksum_fields (string split -m1 ' ' "$checksum_line")
    if test (count $checksum_fields) -lt 1
        echo 'Checksum tool returned no digest.' >&2
        return 1
    end

    set -l checksum $checksum_fields[1]
    if not string match -rq '^[0-9a-f]{64}$' -- "$checksum"
        echo 'Checksum tool returned a non-canonical SHA-256 digest.' >&2
        return 1
    end

    printf '%s\n' "$checksum"
end
