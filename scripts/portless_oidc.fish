#!/usr/bin/env fish

set environment $argv[1]
set route_name $argv[2]

if test -z "$environment"; or test -z "$route_name"
    echo "Usage: scripts/portless_oidc.fish <dev|test> <route-name>" >&2
    exit 64
end

if not type -q portless
    echo "Portless is required for this task." >&2
    echo "Install it globally with: npm install -g portless" >&2
    echo "Then trust its local CA once with: portless trust" >&2
    exit 127
end

set base_url "https://$route_name.localhost"
set callback_url "$base_url/auth/oidc/callback"

if test "$environment" = test
    set -lx TEST_APP_URL $base_url
    set -lx TEST_OIDC_CLIENT_ID $OIDC_CLIENT_ID
    set -lx TEST_OIDC_CLIENT_SECRET $OIDC_CLIENT_SECRET
    set -lx TEST_OIDC_ISSUER_URL $OIDC_ISSUER_URL
    set -lx TEST_OIDC_PROVIDER_NAME $OIDC_PROVIDER_NAME
    set -lx TEST_OIDC_REDIRECT_URI $callback_url
else
    set -lx APP_URL $base_url
    set -lx OIDC_REDIRECT_URI $callback_url
end

set port (task "$environment:port")
if test -z "$port"
    echo "Could not determine the Docker-assigned $environment web port." >&2
    exit 1
end

portless proxy start
portless alias $route_name $port --force

set route_line (portless list | string match -r "https://$route_name\\.localhost[^ ]*")
if not string match -qr "https://$route_name\\.localhost\\s" $route_line
    echo "Portless registered $route_line, not $base_url." >&2
    echo "For Zitadel's clean redirect URI, restart Portless on port 443 from an interactive terminal:" >&2
    echo "  portless proxy stop -p 1355" >&2
    echo "  portless proxy start" >&2
    exit 1
end

echo "Portless URL: $base_url"
echo "OIDC callback: $callback_url"
