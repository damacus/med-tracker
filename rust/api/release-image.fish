set -l root (command git rev-parse --show-toplevel)
or exit 1

set -l platform linux/amd64
set -l rust_target x86_64-unknown-linux-musl
switch (uname -m)
    case arm64 aarch64
        set platform linux/arm64
        set rust_target aarch64-unknown-linux-musl
    case x86_64 amd64
    case '*'
        echo "Unsupported host architecture: "(uname -m) >&2
        exit 1
end

docker buildx build \
    --platform $platform \
    --target runtime \
    --file rust/contract-tests/Dockerfile \
    --build-arg RUST_TARGET=$rust_target \
    --load \
    --tag medtracker-api:local \
    $root
