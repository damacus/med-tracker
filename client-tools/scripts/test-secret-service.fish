#!/usr/bin/env fish

if test (uname) != Linux
    printf 'private Secret Service verification requires Linux\n' >&2
    exit 1
end

function interrupt_secret_service --on-signal INT --on-signal TERM
    exit 130
end

if test "$argv[1]" != --inside
    for tool in dbus-run-session dbus-send gnome-keyring-daemon secret-tool timeout
        if not command -sq $tool
            printf 'missing Secret Service test prerequisite: %s\n' $tool >&2
            exit 1
        end
    end

    set -g secret_service_root (mktemp -d)
    or exit $status
    chmod 700 $secret_service_root
    mkdir -p $secret_service_root/data $secret_service_root/config $secret_service_root/runtime
    chmod 700 $secret_service_root/runtime

    function cleanup_secret_service_root --on-event fish_exit
        if set -q secret_service_root
            rm -rf -- $secret_service_root
        end
    end

    set -gx XDG_DATA_HOME $secret_service_root/data
    set -gx XDG_CONFIG_HOME $secret_service_root/config
    set -gx XDG_RUNTIME_DIR $secret_service_root/runtime
    set -gx MEDTRACKER_PRIVATE_SECRET_SERVICE 1
    set -e MEDTRACKER_TOKEN
    set -e GNOME_KEYRING_CONTROL GNOME_KEYRING_PID
    dbus-run-session -- fish (status filename) --inside
    exit $status
end

if not set -q DBUS_SESSION_BUS_ADDRESS XDG_DATA_HOME XDG_CONFIG_HOME XDG_RUNTIME_DIR MEDTRACKER_PRIVATE_SECRET_SERVICE
    printf 'private Secret Service environment is incomplete\n' >&2
    exit 1
end

set -e GNOME_KEYRING_CONTROL GNOME_KEYRING_PID MEDTRACKER_TOKEN
function cleanup_secret_service_daemon --on-event fish_exit
    if set -q secret_service_daemon_pid[1]; and string match -rq '^[0-9]+$' -- $secret_service_daemon_pid
        kill $secret_service_daemon_pid 2>/dev/null
        wait $secret_service_daemon_pid 2>/dev/null
    end
end

mkdir -p $XDG_RUNTIME_DIR/keyring
printf 'synthetic-unlock' | gnome-keyring-daemon --foreground --unlock --components=secrets --control-directory=$XDG_RUNTIME_DIR/keyring > /dev/null 2>&1 &
set secret_service_daemon_pid $last_pid

set -l ready 0
for attempt in (seq 1 40)
    if not kill -0 $secret_service_daemon_pid 2>/dev/null
        break
    end
    if dbus-send --session --print-reply --reply-timeout=1000 --dest=org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.DBus.Peer.Ping > /dev/null 2>&1
        set ready 1
        break
    end
    sleep 0.25
end
if test $ready -ne 1
    printf 'private GNOME Secret Service did not become ready\n' >&2
    exit 1
end

if not printf 'synthetic-ready' | timeout 10s secret-tool store --collection=default --label='MedTracker private readiness' service medtracker-test-ready username readiness > /dev/null 2>&1
    printf 'private Secret Service default collection could not be written\n' >&2
    exit 1
end
set -l probe (timeout 10s secret-tool lookup service medtracker-test-ready username readiness 2>/dev/null)
if test $status -ne 0; or test "$probe" != synthetic-ready
    printf 'private Secret Service default collection could not be read\n' >&2
    exit 1
end
timeout 10s secret-tool clear service medtracker-test-ready username readiness > /dev/null 2>&1
or exit $status

timeout 180s cargo nextest run --manifest-path Cargo.toml --locked -p medtracker-cli --all-features --run-ignored all --no-tests=fail -E 'test(secrets::tests::existing_secret_tool_item_is_interoperable)'
exit $status
