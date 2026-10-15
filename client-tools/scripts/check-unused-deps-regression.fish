#!/usr/bin/env fish

set -g fake_bin (mktemp -d)
or exit $status

function cleanup --on-event fish_exit
    if test -n "$fake_bin"
        command rm -rf -- "$fake_bin"
    end
end

printf '%s\n' \
    '#!/usr/bin/env fish' \
    'switch $argv[1]' \
    '    case metadata' \
    '        if test "$FAKE_CARGO_CASE" = metadata_failure' \
    '            exit 9' \
    '        end' \
    '    case machete' \
    '        printf touched > "$FAKE_CARGO_MACHETE_MARKER"' \
    '        switch "$FAKE_CARGO_CASE"' \
    '            case partial' \
    '                echo "error when handling ./crates/example/Cargo.toml: cargo metadata exited with an error"' \
    '                exit 0' \
    '            case clean' \
    '                echo "cargo-machete did not find any unused dependencies"' \
    '                exit 0' \
    '            case tool_failure' \
    '                exit 7' \
    '        end' \
    'end' > "$fake_bin/cargo"
or exit $status
chmod +x "$fake_bin/cargo"
or exit $status

function run_case --argument-names scenario expected_status
    set -l marker "$fake_bin/machete-invoked"
    command rm -f -- "$marker"
    set -lx PATH "$fake_bin" $PATH
    set -lx FAKE_CARGO_CASE "$scenario"
    set -lx FAKE_CARGO_MACHETE_MARKER "$marker"
    set -l output (fish --no-config scripts/check-unused-deps.fish 2>&1)
    set -l result $status

    if test "$scenario" = partial
        if test $result -eq 0
            printf 'machete partial analysis was accepted\n' >&2
            return 1
        end
        if not string match -q -- '*analysis error*' (string join ' ' $output)
            printf 'machete partial analysis did not report the error\n' >&2
            printf '%s\n' $output >&2
            return 1
        end
    else if test $result -ne $expected_status
        printf 'machete %s returned %s, expected %s\n' "$scenario" "$result" "$expected_status" >&2
        return 1
    end

    if test "$scenario" = metadata_failure
        if test -e "$marker"
            printf 'machete ran after metadata validation failed\n' >&2
            return 1
        end
    else if not test -e "$marker"
        printf 'machete did not run in %s case\n' "$scenario" >&2
        return 1
    end
end

run_case partial 1
or exit $status
run_case clean 0
or exit $status
run_case tool_failure 7
or exit $status
run_case metadata_failure 9
or exit $status
