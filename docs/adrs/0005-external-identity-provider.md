# ADR 0005: External Identity Provider for Web and Mobile Authentication

- Status: accepted, with mobile PKCE delivery incomplete
- Date: 2026-07-02

## Context

MedTracker supports local Rodauth sign-in and optional browser sign-in through
an OpenID Connect provider. It also provides internal API access and refresh
tokens.

A hosted mobile client should not collect a MedTracker password. It needs a
provider-managed login while MedTracker keeps control of household access, API
session revocation, and audit evidence.

## Decision

Hosted deployments can use an external OpenID Connect provider as the primary
identity service. Zitadel is the preferred provider for MedTracker-operated
deployments, but the browser integration remains provider-neutral.

The web application remains a Rodauth OIDC client. Rodauth owns the browser
session, account linking, and MedTracker login lifecycle after the provider
authenticates the person.

First-party mobile clients use MedTracker's Rodauth authorization code flow with
S256 PKCE in the system browser or platform authentication session. Rodauth offers
the installation's configured local and delegated login methods. The external
OIDC exchange remains inside the browser authentication boundary.

Mobile credentials identify the account. Each household request resolves current
membership and person/action permissions; choosing a household does not require
another login. Restricted integration credentials retain their household limits.

MedTracker remains responsible for:

- account and provider-subject linking;
- household membership and person access;
- internal access and refresh token rotation;
- session revocation and lockout checks;
- privacy-safe audit evidence.

## Current delivery status

The browser authorization code flow is available and is documented in
[OpenID Connect setup](../oidc-setup.md).

PR #2232 replaced the custom mobile ID-token exchange, password login,
household-selection exchange and refresh routes. Clients discover MedTracker's
authorization server and public mobile client configuration through
`/.well-known/oauth-authorization-server` and `/api/v1/capabilities`. Rodauth owns
code redemption and refresh rotation; phones do not need a ZITADEL native client
ID or the retired exchange's `OIDC_MOBILE_CLIENT_ID` setting.

Issue #1889 remains open for failure-path, provider-backed login and rollout
acceptance. Merged source and passing local tests do not establish deployed
ZITADEL or passkey behaviour. Local browser password login remains available
where installation policy permits it.

## Security requirements

The completed mobile flow must:

- register public mobile clients at MedTracker with exact platform callbacks;
- use an exact redirect URI for each client and environment;
- verify issuer, audience, expiry, nonce, signature, and the S256 PKCE
  relationship;
- store the provider subject in `AccountIdentity`;
- keep provider roles separate from MedTracker access policy;
- exclude tokens, codes, verifiers, and health data from logs and audit fields.

## Consequences

- Browser and mobile clients can share an identity provider without sharing
  their client credentials.
- MedTracker owns account-level mobile grant revocation and per-request household
  authorisation, while restricted integrations retain their existing limits.
- Mobile login cannot be declared complete until #1889 is resolved.
- Local password authentication remains available where the deployment policy
  permits it.

## Related documents

- [OpenID Connect setup](../oidc-setup.md)
- [Test local MedTracker with Zitadel](../zitadel-local-testing.md)
- [Authentication and authorization](0002-authentication-and-authorization-strategy.md)
- [External integration architecture](0010-external-integration-architecture.md)
