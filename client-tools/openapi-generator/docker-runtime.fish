function openapi_generator_docker_run
    if not type -q id
        echo 'Unable to determine the invoking user because id is unavailable.' >&2
        return 1
    end

    set -l host_uid (id -u)
    set -l uid_status $status
    set -l host_gid (id -g)
    set -l gid_status $status

    if test $uid_status -ne 0; or test $gid_status -ne 0
        echo 'Unable to determine the invoking numeric UID and GID.' >&2
        return 1
    end

    if test (count $host_uid) -ne 1; or not string match -rq '^[0-9]+$' -- "$host_uid"
        echo 'The invoking UID is not a single numeric value.' >&2
        return 1
    end

    if test (count $host_gid) -ne 1; or not string match -rq '^[0-9]+$' -- "$host_gid"
        echo 'The invoking GID is not a single numeric value.' >&2
        return 1
    end

    docker run --user "$host_uid:$host_gid" $argv
end
