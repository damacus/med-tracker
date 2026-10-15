#!/usr/bin/env fish

set -l output (task --taskfile Taskfile.yml --dry ci 2>&1)
if test $status -ne 0
    printf '%s\n' $output
    exit 1
end

set -l commands (string join ' ' $output)
for expected in \
    'cargo fmt --manifest-path Cargo.toml --all -- --check' \
    'cargo clippy --manifest-path Cargo.toml --locked --workspace --all-targets --all-features -- -D warnings' \
    'cargo nextest run --manifest-path Cargo.toml --locked --workspace --all-features --no-tests=fail' \
    'cargo test --manifest-path Cargo.toml --locked --workspace --all-features --doc' \
    'cargo deny --manifest-path Cargo.toml --config ../deny.toml --workspace --locked --all-features check advisories bans sources'
    if not string match -q -- "*$expected*" $commands
        printf 'missing gate command: %s\n' $expected
        exit 1
    end
end

if not string match -rq -- 'cargo fmt.*cargo clippy.*fish scripts/check-unused-deps.fish.*cargo deny.*cargo nextest.*cargo test .*--doc' $commands
    printf 'gate order differs from fmt, clippy, machete, deny, nextest, doc tests\n'
    exit 1
end

set -l machete_script (string collect < scripts/check-unused-deps.fish)
for expected in \
    'cargo metadata --manifest-path Cargo.toml --locked --no-deps --format-version 1' \
    'cargo machete --with-metadata --skip-target-dir .'
    if not string match -q -- "*$expected*" $machete_script
        printf 'missing scoped machete step: %s\n' $expected
        exit 1
    end
end

set -l check_output (task --taskfile Taskfile.yml --dry check 2>&1)
if test $status -ne 0
    printf '%s\n' $check_output
    exit 1
end

if not string match -q -- '*cargo check --manifest-path Cargo.toml --locked --workspace --all-targets --all-features*' (string join ' ' $check_output)
    printf 'check gate is missing locked all-targets all-features coverage\n'
    exit 1
end
