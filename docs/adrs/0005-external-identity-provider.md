# ADR 0005: External Identity Provider for Web and Mobile Authentication

- Status: Accepted
- Date: 2026-07-02

## Context

MedTracker already supports web sign-in through Rodauth and an OIDC provider.
It also exposes `POST /api/v1/auth/login`, which exchanges an email and
password for internal API access and refresh tokens.

That password-to-token endpoint is acceptable for internal transition work, but
it should not become the primary authentication model for first-party mobile
clients. A hosted deployment needs a dedicated identity provider for primary
authentication, MFA, passkeys, recovery, and OAuth/OIDC client policy.

## Decision

External IdP owns primary authentication. Zitadel is the preferred provider for
local and hosted deployments, but the integration remains OIDC-provider
compatible.

The web app remains a Rodauth OIDC client. Rodauth continues to own the browser
session, account linking, and MedTracker-specific login lifecycle after the
provider has authenticated the user.

First-party mobile clients use Authorization Code with PKCE against the external
provider; in other words, mobile clients use Authorization Code with PKCE.
The mobile app must open the system browser or platform authentication session,
complete the provider-managed login, and return with an authorization code
through an app link, universal link, or custom scheme registered for that client.

MedTracker will exchange external identity for internal API sessions. The API
will validate the provider result server-side, link it to an internal account,
and mint MedTracker access and refresh tokens backed by `ApiSession`.

## Rationale

Keeping internal API sessions after external authentication fits the current
`ApiSession` model and preserves MedTracker-owned revocation, audit logging, and
token rotation behavior.

Directly accepting provider-issued access tokens at every API endpoint would
push issuer, audience, scope, and key-rotation checks into the resource-server
path before the rest of the API is ready for that responsibility.

## Implementation Notes

- Register separate OIDC clients for the web app and each mobile platform.
- Keep redirect URI configuration explicit per client and environment.
- Require issuer, audience, expiry, nonce, and signature validation before
  linking or provisioning a MedTracker account.
- Store the provider subject through `AccountIdentity`; do not key access on an
  email address alone.
- Keep role, household, and person authorization in MedTracker policies.
- Do not log provider tokens, authorization codes, medication data, or health
  event payloads during the exchange.

## Password Login Deprecation

`POST /api/v1/auth/login` is deprecated as a first-party mobile authentication
entrypoint.

During transition it may remain available for local development, tests, or
migration-only clients. Production mobile clients should move to external OIDC
plus internal API session exchange before the endpoint is restricted further or
removed.

## Consequences

### Positive

- Authentication policy, MFA, recovery, and passkeys live in the external IdP.
- Mobile clients avoid collecting MedTracker passwords directly.
- MedTracker keeps existing API session revocation and audit behavior.
- Web and mobile clients share one identity authority.

### Negative

- Mobile login requires provider client registration and redirect URI handling.
- The API needs a new exchange endpoint before password login can be removed.
- Account linking and provisioning rules must be tested carefully.

## Follow-up Work

- Add the mobile identity-to-session exchange endpoint.
- Register first-party mobile clients and document redirect URI conventions.
- Add request specs for invalid issuer, audience, expiry, replay, and revoked
  session behavior.
- Restrict `POST /api/v1/auth/login` once mobile clients no longer depend on it.

## Related Documents

- `docs/oidc-setup.md`
- `docs/zitadel-local-testing.md`
- `docs/adrs/0002-authentication-and-authorization-strategy.md`
- `app/misc/rodauth_main.rb`
- `app/models/account_identity.rb`
- `app/models/api_session.rb`
