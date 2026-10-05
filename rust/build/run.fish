if test (count $argv) -eq 0
    echo 'Usage: fish rust/build/run.fish <command> [arguments...]' >&2
    exit 2
end

set -l root (command git rev-parse --show-toplevel)
or exit 1
set -l current_dir (pwd)
set -l relative_dir .
if test "$current_dir" != "$root"
    set relative_dir (string replace "$root/" '' -- "$current_dir")
end
set -l image medtracker-rust-toolchain:1.99.0
set -l user_id (id -u)
set -l group_id (id -g)

if not docker image inspect $image >/dev/null 2>&1
    docker build --file "$root/rust/build/Dockerfile" --tag $image "$root/rust/build"
    or exit 1
end

command mkdir -p "$root/tmp/rust-cargo-home" "$root/tmp/npm-cache"
or exit 1

docker run --rm \
    --user "$user_id:$group_id" \
    --env CARGO_HOME=/workspace/tmp/rust-cargo-home \
    --env CARGO_BUILD_JOBS=2 \
    --env npm_config_cache=/workspace/tmp/npm-cache \
    --volume "$root:/workspace" \
    --workdir "/workspace/$relative_dir" \
    $image $argv
