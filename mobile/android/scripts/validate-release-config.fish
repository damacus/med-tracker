#!/usr/bin/env fish

set -l required_values \
    MEDTRACKER_RELEASE_SERVER_URL \
    MEDTRACKER_RELEASE_OIDC_AUTHORIZATION_ENDPOINT \
    MEDTRACKER_RELEASE_OIDC_TOKEN_ENDPOINT \
    MEDTRACKER_RELEASE_OIDC_CLIENT_ID \
    MEDTRACKER_RELEASE_OIDC_REDIRECT_URI \
    MEDTRACKER_RELEASE_OIDC_REDIRECT_SCHEME

for variable_name in $required_values
    if not set -q $variable_name
        echo "$variable_name is required" >&2
        exit 2
    end
    set -l value $$variable_name
    if test -z "$value"; or string match --quiet --ignore-case '*invalid*' "$value"
        echo "$variable_name must contain an explicit non-placeholder value" >&2
        exit 2
    end
end

for variable_name in MEDTRACKER_RELEASE_SERVER_URL MEDTRACKER_RELEASE_OIDC_AUTHORIZATION_ENDPOINT MEDTRACKER_RELEASE_OIDC_TOKEN_ENDPOINT
    if not string match --quiet 'https://*' "$$variable_name"
        echo "$variable_name must use https" >&2
        exit 2
    end
end

if not string match --quiet "$MEDTRACKER_RELEASE_OIDC_REDIRECT_SCHEME:*" "$MEDTRACKER_RELEASE_OIDC_REDIRECT_URI"
    echo "MEDTRACKER_RELEASE_OIDC_REDIRECT_URI must use MEDTRACKER_RELEASE_OIDC_REDIRECT_SCHEME" >&2
    exit 2
end
