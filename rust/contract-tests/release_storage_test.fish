command mkdir -p tmp/contract-tests
or exit $status
set -l run_dir (mktemp -d tmp/contract-tests/run.XXXXXX)
or exit $status
set -l project mtcontract-(string lower (string replace -a - '' (uuidgen)) | string sub -l 16)
echo $project >$run_dir/owner
mkdir $run_dir/storage $run_dir/shim
set -l storage (realpath $run_dir/storage)
set -l image medtracker-contract-runner:$project
set -l volume $project-storage
set -l docker_bin (command -s docker)
docker pull rust:1.99.0-bookworm >/dev/null
or exit $status
docker tag rust:1.99.0-bookworm $image
or exit $status
docker volume create $volume >/dev/null
or exit $status
docker run --rm --network none --mount "type=volume,source=$volume,target=/storage" $image install -d -m 700 -o 12345 -g 12345 /storage/private
or exit $status
docker run --rm --network none --mount "type=volume,source=$volume,target=/storage" $image install -m 600 -o 12345 -g 12345 /dev/null /storage/private/avatar
or exit $status
printf '%s\n' '#!/usr/bin/env fish' 'set -l mapped_args' 'for arg in $argv' 'if test "$arg" = "$CONTRACT_TEST_BIND"' 'set -a mapped_args "type=volume,source=$CONTRACT_TEST_VOLUME,target=/storage"' 'else' 'set -a mapped_args "$arg"' 'end' 'end' 'command $CONTRACT_TEST_DOCKER $mapped_args' >$run_dir/shim/docker
chmod +x $run_dir/shim/docker
set -lx CONTRACT_TEST_BIND "type=bind,source=$storage,target=/storage"
set -lx CONTRACT_TEST_VOLUME $volume
set -lx CONTRACT_TEST_DOCKER $docker_bin
set -lx CONTRACT_PROJECT $project
set -lx CONTRACT_RUN_DIR $run_dir
set -lx PATH (realpath $run_dir/shim) $PATH
echo mtcontract-0000000000000000 >$run_dir/owner
fish --no-config rust/contract-tests/release_storage.fish >/dev/null 2>&1
set -l invalid_status $status
echo $project >$run_dir/owner
fish --no-config rust/contract-tests/release_storage.fish
set -l released $status
set -l owner (docker run --rm --network none --mount "type=volume,source=$volume,target=/storage" $image stat -c '%u:%g:%a' /storage/private/avatar)
set -l expected (id -u):(id -g):600
docker volume rm $volume >/dev/null
docker image rm $image >/dev/null
rm -r $run_dir
test $invalid_status -ne 0; and test $released -eq 0; and test "$owner" = "$expected"
or begin; echo "Private avatar ownership was not restored: $owner; expected $expected" >&2; exit 1; end
echo 'Linux private avatar ownership restored without changing file permissions; invalid owner refused'
