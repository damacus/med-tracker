# OpenID Connect Setup

MedTracker can use an OpenID Connect (OIDC) provider for browser sign-in. The
provider must publish discovery metadata and support the authorization code
flow.

This guide covers the server-side browser flow. The hosted mobile API exchange
is a separate contract and is not part of this setup.

## Browser flow

MedTracker uses Rodauth, OmniAuth, and `omniauth_openid_connect`. The browser
flow requests the `openid`, `email`, and `profile` scopes.

1. A person selects **Continue with _provider_** on the login page.
2. MedTracker redirects the browser to the provider.
3. The provider returns an authorization code to MedTracker.
4. MedTracker exchanges the code on the server and validates the returned
   identity.
5. MedTracker links the provider identity to an eligible account.

The provider does not grant household roles or person access. MedTracker keeps
those decisions in household memberships and person access grants.

## Provider registration

Create a confidential web application in the provider. Register this callback:

```text
https://medtracker.example.com/auth/oidc/callback
```

Use the exact public MedTracker origin. The scheme, host, port, and path must
match the configured redirect URI.

The provider must supply:

- an HTTPS issuer URL with OpenID discovery metadata;
- a client ID;
- a client secret;
- the subject and email claims requested by the configured scopes.

HTTP issuer and redirect URLs are accepted only for `localhost` development.

## MedTracker configuration

Set these required values:

```fish
set -x APP_URL "https://medtracker.example.com"
set -x OIDC_ISSUER_URL "https://identity.example.com"
set -x OIDC_CLIENT_ID "your-client-id"
set -x OIDC_CLIENT_SECRET "your-client-secret"
```

`APP_URL` is required in production. MedTracker derives the callback as
`$APP_URL/auth/oidc/callback`.

These optional values change the callback or the login button label:

```fish
set -x OIDC_REDIRECT_URI "https://medtracker.example.com/auth/oidc/callback"
set -x OIDC_PROVIDER_NAME "Your identity provider"
```

You can store the OIDC provider settings in Rails encrypted credentials
instead:

```yaml
oidc:
  issuer_url: https://identity.example.com
  client_id: your-client-id
  client_secret: your-client-secret
```

Environment variables and encrypted credentials are both supported. Never put
the client secret in source files, container images, or public logs.

## Local development

For Portless development, register this callback with the provider:

```text
https://med-tracker.localhost/auth/oidc/callback
```

Set the stable URLs before you start MedTracker:

```fish
set -x APP_URL "https://med-tracker.localhost"
set -x OIDC_REDIRECT_URI "https://med-tracker.localhost/auth/oidc/callback"
set -x OIDC_ISSUER_URL "https://identity.example.com"
set -x OIDC_CLIENT_ID "your-client-id"
set -x OIDC_CLIENT_SECRET "your-client-secret"
task dev:portless
```

The issuer must be reachable from the MedTracker container. A host-only
`localhost` issuer will not work unless the container can resolve and reach it.

For Zitadel-specific steps, see
[Test local MedTracker with Zitadel](zitadel-local-testing.md).

## Account and access rules

MedTracker can link a provider identity to an existing account with the same
email. New accounts remain subject to the registration policy. When
invitation-only registration is active, an uninvited provider identity cannot
create an account.

An inactive account or a person without an active household membership cannot
use OIDC to bypass those restrictions.

Zitadel project roles named `doctor` or `nurse` can update a person's
professional title. They do not become MedTracker household roles.

## Verification

After configuration:

1. Open `/login` and confirm that the provider button appears.
2. Sign in with an invited or existing account.
3. Confirm that the provider returns to `/auth/oidc/callback`.
4. Confirm that MedTracker opens the expected household dashboard.
5. Sign out and confirm that the local session ends.

Do not use a production account to test an untrusted or development provider.

## Troubleshooting

### Provider button is missing

Both `OIDC_CLIENT_ID` and `OIDC_ISSUER_URL` must be present. Restart the service
after changing its environment.

### Redirect URI mismatch

Compare the provider registration with the effective MedTracker callback. Check
the scheme, host, port, and `/auth/oidc/callback` path.

### Discovery fails

Confirm that this URL returns valid provider metadata and is reachable from the
MedTracker container:

```text
https://identity.example.com/.well-known/openid-configuration
```

### Production reports an insecure URL

Production issuer and redirect URLs must use HTTPS. HTTP is allowed only for
`localhost`, `127.0.0.1`, and `::1`.

## Related documentation

- [Test local MedTracker with Zitadel](zitadel-local-testing.md)
- [Authentication and authorization](adrs/0002-authentication-and-authorization-strategy.md)
- [External identity provider decision](adrs/0005-external-identity-provider.md)
