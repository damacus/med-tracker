#!/usr/bin/env fish

set -l script_dir (dirname (status filename))
set -l image 'openapitools/openapi-generator-cli:v7.20.0@sha256:fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
source "$script_dir/docker-runtime.fish"

set -l test_root (mktemp -d)
if test $status -ne 0
    exit 1
end
set -g openapi_generator_docker_user_test_root "$test_root"

function cleanup --on-event fish_exit
    if set -q openapi_generator_docker_user_test_root
        set -l root "$openapi_generator_docker_user_test_root"
        set -e openapi_generator_docker_user_test_root
        if test -d "$root"
            rm -rf "$root"
        end
    end
end

set -l host_uid (id -u)
set -l host_gid (id -g)
set -l host_owner "$host_uid:$host_gid"
if not openapi_generator_docker_run --rm \
        --entrypoint sh \
        -v "$test_root:/output" \
        $image \
        -c 'mkdir -p /output/generated && id -u > /output/generated/container-uid && id -g > /output/generated/container-gid && touch /output/generated/client.txt'
    echo 'The Docker UID/GID bind-mount test failed.' >&2
    exit 1
end

set -l generated_dir "$test_root/generated"
set -l generated_file "$generated_dir/client.txt"
set -l container_uid_file "$generated_dir/container-uid"
set -l container_gid_file "$generated_dir/container-gid"
read -l container_uid < "$container_uid_file"
read -l container_gid < "$container_gid_file"

if test "$container_uid" != "$host_uid"; or test "$container_gid" != "$host_gid"
    echo "Container user $container_uid:$container_gid does not match invoking user $host_owner." >&2
    exit 1
end

if not test -w "$generated_file"
    echo 'Generated file is not writable by the invoking user.' >&2
    exit 1
end

if test (uname -s) = Linux
    set -l generated_owner (stat -c '%u:%g' "$generated_file")
    if test "$generated_owner" != "$host_owner"
        echo "Generated file owner $generated_owner does not match invoking user $host_owner." >&2
        exit 1
    end
end

if not rm "$generated_file" "$container_uid_file" "$container_gid_file"; or not rmdir "$generated_dir"
    echo 'Generated file could not be removed by the invoking user.' >&2
    exit 1
end

if test -e "$generated_file"
    echo 'Generated file still exists after removal.' >&2
    exit 1
end

echo "docker_user=$host_owner"
echo 'docker_user_permissions=passed'
