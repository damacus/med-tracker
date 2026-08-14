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

First-party mobile clients will use the authorization code flow with S256 PKCE
against the external provider. The mobile client will use the system browser or
platform authentication session. MedTracker will validate the provider result
and create an internal API session linked to the eligible account and household
membership.

MedTracker remains responsible for:

- account and provider-subject linking;
- household membership and person access;
- internal access and refresh token rotation;
- session revocation and lockout checks;
- privacy-safe audit evidence.

## Current delivery status

The browser authorization code flow is available and is documented in
[OpenID Connect setup](../oidc-setup.md).

The mobile API exposes an ID-token exchange, but it does not yet verify the
relationship between `code_verifier` and an S256 challenge. Issue #1889 tracks
the missing security contract. Until that issue is complete, clients and API
documentation must not describe the endpoint as a complete PKCE exchange.

The password-based API login remains available for development, tests, and
transition clients. Restricting it requires a separate compatibility decision.

## Security requirements

The completed mobile flow must:

- register a separate provider client for each mobile platform;
- use an exact redirect URI for each client and environment;
- verify issuer, audience, expiry, nonce, signature, and the S256 PKCE
  relationship;
- store the provider subject in `AccountIdentity`;
- keep provider roles separate from MedTracker access policy;
- exclude tokens, codes, verifiers, and health data from logs and audit fields.

## Consequences

- Browser and mobile clients can share an identity provider without sharing
  their client credentials.
- MedTracker keeps its existing API session and revocation model.
- Mobile login cannot be declared complete until #1889 is resolved.
- Local password authentication remains available where the deployment policy
  permits it.

## Related documents

- [OpenID Connect setup](../oidc-setup.md)
- [Test local MedTracker with Zitadel](../zitadel-local-testing.md)
- [Authentication and authorization](0002-authentication-and-authorization-strategy.md)
- [External integration architecture](0010-external-integration-architecture.md)
