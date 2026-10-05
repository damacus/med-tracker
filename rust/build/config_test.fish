set -l root (git rev-parse --show-toplevel)
set -l failures 0

function require_match -a path pattern
    if not rg -q --fixed-strings -- "$pattern" "$path"
        echo "Expected '$pattern' in $path" >&2
        set -g failures (math $failures + 1)
    end
end

require_match "$root/rust/contract-tests/Dockerfile" 'FROM scratch AS runtime'
require_match "$root/rust/contract-tests/Dockerfile" 'linux-musl'
require_match "$root/rust/contract-tests/Dockerfile" 'FROM dependencies AS runner'
require_match "$root/rust/contract-tests/runner.compose.yaml" 'target: runner'
require_match "$root/rust/api/Taskfile.yml" 'fish rust/api/release-image.fish'
require_match "$root/rust/api/Taskfile.yml" 'docker compose -p {{ .CONTRACT_PROJECT }} --profile test build rust-api'
require_match "$root/rust/api/Taskfile.yml" 'fish rust/build/run.fish cargo build'
require_match "$root/rust/web/Taskfile.yml" 'fish ../build/run.fish cargo'
require_match "$root/rust/ui-preview/Taskfile.yml" 'fish ../build/run.fish cargo'
require_match "$root/rust/build/Dockerfile" 'FROM rust:1.99.0-bookworm'
require_match "$root/rust/build/Dockerfile" 'rustup component add clippy rustfmt'
require_match "$root/rust/build/run.fish" '--env CARGO_BUILD_JOBS=2'

if test "$failures" -gt 0
    exit 1
end
