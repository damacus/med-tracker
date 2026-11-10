# OpenID Connect setup

MedTracker uses Better Auth’s generic OAuth/OIDC plugin for optional provider sign-in. Configure one OpenID Connect provider for the installation; the provider does not grant household membership or clinical access.

## Configuration

Supply these settings through your deployment secret store and Loco configuration:

| Variable | Purpose |
| --- | --- |
| `OIDC_ISSUER_URL` | HTTPS issuer publishing OpenID discovery metadata. |
| `OIDC_CLIENT_ID` | Confidential web application client ID. |
| `OIDC_CLIENT_SECRET` | Server-side client secret; never expose it to browsers or mobile clients. |
| `OIDC_REDIRECT_URI` | Optional explicit callback URL; defaults to the public origin plus `/auth/oidc/callback`. |
| `OIDC_PROVIDER_NAME` | Display branding, such as “Your organisation”; defaults to “OpenID Connect”. |
| `MEDTRACKER_PUBLIC_ORIGIN` | Canonical public application origin, including HTTPS scheme and any public port. |

With no issuer configured, provider sign-in is disabled. A configured issuer requires both client credentials. The display name affects interface text only; it does not change the stored provider identity.

For example, configure a provider with issuer `https://identity.example.com` and the public application origin `https://medtracker.example.com`. Register this callback on its confidential web client:

```text
https://medtracker.example.com/auth/oidc/callback
```

An explicit callback must use the same public origin and HTTPS, with no credentials, query or fragment. MedTracker registers its configured callback path and passes that exact URI to Better Auth. Local HTTP is restricted to local test/development origins. Provider sign-in starts at `/auth/oidc`; provider registration uses `/auth/oidc/register`.

## Provider and application responsibilities

The provider publishes discovery metadata and supports the authorization code flow with the requested `openid`, `email` and `profile` scopes. Better Auth owns discovery, code exchange, PKCE, nonce binding and ID-token validation. MedTracker supplies configuration and adapts native endpoints to its browser pages.

Existing accounts link the provider explicitly through **Account security**. Matching email addresses alone do not link accounts. New registrations collect the required name, adult date of birth and invitation details before provider sign-in and remain subject to the installation’s registration policy. Local sign-in methods remain available according to the account’s configured methods.

MedTracker retains its CSRF and origin checks, permission checks, audit, session revocation and household access rules. Changing provider branding does not change these rules.

## Compose and Helm

Set the same `OIDC_*` values on the application processes in Docker Compose or Helm values. Store client credentials in the platform’s Secret mechanism. HomeOps resolves its existing client through 1Password and ExternalSecrets; other installations can use their own provider and secret store.

## Mobile and verification

First-party mobile clients discover MedTracker’s authorization server through `/.well-known/oauth-authorization-server` and `/api/v1/capabilities`. They use MedTracker’s authorization code flow with S256 PKCE in the system browser. The external provider’s confidential secret remains on the server.

Verify registration, sign-in, explicit linking/unlinking, reauthentication, callback rejection and household access against the intended provider. Verify the actual mobile return and token refresh separately; a successful browser login does not establish the complete phone journey.
