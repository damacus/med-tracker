#!/usr/bin/env fish

cargo metadata --manifest-path Cargo.toml --locked --no-deps --format-version 1 > /dev/null
or exit $status

set -l output (cargo machete --with-metadata --skip-target-dir . 2>&1)
set -l result $status
printf '%s\n' $output

if test $result -ne 0
    exit $result
end

if string match -riq -- '(^|[^[:alpha:]])(error|failed|could not|cannot|permission denied)([^[:alpha:]]|$)' (string join ' ' $output)
    printf 'cargo machete reported an analysis error\n' >&2
    exit 1
end
