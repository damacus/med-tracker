#!/usr/bin/env fish

set -l script_dir (dirname (status filename))

fish --no-config "$script_dir/validate-release-config.fish"
or exit 2

./gradlew :phone:assembleRelease --no-daemon \
    "-Pmedtracker.release.serverUrl=$MEDTRACKER_RELEASE_SERVER_URL" \
    "-Pmedtracker.release.oidcAuthorizationEndpoint=$MEDTRACKER_RELEASE_OIDC_AUTHORIZATION_ENDPOINT" \
    "-Pmedtracker.release.oidcTokenEndpoint=$MEDTRACKER_RELEASE_OIDC_TOKEN_ENDPOINT" \
    "-Pmedtracker.release.oidcClientId=$MEDTRACKER_RELEASE_OIDC_CLIENT_ID" \
    "-Pmedtracker.release.oidcRedirectUri=$MEDTRACKER_RELEASE_OIDC_REDIRECT_URI" \
    "-Pmedtracker.release.oidcRedirectScheme=$MEDTRACKER_RELEASE_OIDC_REDIRECT_SCHEME"
or exit 1

fish --no-config "$script_dir/verify-release-security.fish"
