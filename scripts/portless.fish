#!/usr/bin/env fish
set action $argv[1]
set name $argv[2]
set origin $argv[3]
set old_port $argv[4]
set binding $argv[5]
if test "$PORTLESS" = 0
    if test "$action" = register
        echo "$name ready at http://$binding"
    end
    exit 0
end
set cli (command -s portless)
if test -z "$cli"; and command -q mise
    set cli (mise which portless)
end
if test -z "$cli"
    echo 'Portless is missing. Run mise install npm:portless, then portless doctor; or use PORTLESS=0.' >&2
    exit 127
end
set -lx PORTLESS_SYNC_HOSTS 0
set -lx PORTLESS_LAN 0
set -lx NO_COLOR 1
set proxy_port 443
set -q PORTLESS_PORT; and set proxy_port $PORTLESS_PORT
set expected "https://$name.localhost"
test "$proxy_port" = 443; or set expected "$expected:$proxy_port"
set diagnostics ($cli doctor | string collect)
or exit $status
if not string match -rq '^Mode:\s+HTTPS,\s+\.localhost$' -- (string split \n $diagnostics)
    echo 'Portless needs an existing trusted HTTPS proxy exclusively for .localhost. Run portless doctor, or use PORTLESS=0.' >&2
    exit 1
end
set routes ($cli list | string collect)
or exit $status
set route (string split \n $routes | string match -r "^\\s*https://$name\\.localhost(?::$proxy_port)?\\s+->\\s+localhost:([0-9]+)\\s+\\(alias\\)" | tail -n 1)
if test -z "$route"; and string match -q "*$name.localhost*" -- $routes
    echo "Refusing non-static Portless route for $name" >&2
    exit 1
end
switch $action
    case check
        if test -n "$route"; and test "$route" != "$old_port"
            echo "Refusing alias that does not match the current owned Compose port: $name" >&2
            exit 1
        end
        test ($cli get $name --no-worktree) = "$expected"; or exit 1
        set headers (curl --silent --show-error --head --max-time 3 --resolve "$name.localhost:$proxy_port:127.0.0.1" $expected | string collect)
        or exit $status
        string match -riq 'x-portless: 1' -- $headers; or exit 1
    case register
        set port (string split : $binding)[-1]
        if test -n "$route"; and test "$route" != "$old_port"; and test "$route" != "$port"
            echo "Refusing alias owned by another app: $name" >&2
            exit 1
        end
        $cli alias $name $port >/dev/null
        or exit $status
        curl --fail --silent --show-error --max-time 3 --retry 3 --retry-delay 1 --retry-all-errors --retry-max-time 5 --resolve "$name.localhost:$proxy_port:127.0.0.1" "$expected/health" >/dev/null
        or exit $status
        echo "$name ready at $expected"
    case remove
        if test -z "$route"
            exit 0
        end
        if test -z "$old_port"; or test "$route" != "$old_port"
            echo "Alias does not match a running owned app. Inspect before manual cleanup: portless alias --remove $name" >&2
            exit 1
        end
        $cli alias --remove $name
    case '*'
        echo 'Usage: portless.fish check|register|remove name origin old-port [binding]' >&2
        exit 64
end
