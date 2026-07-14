# ADR 0010: External Integration Architecture

- Status: Accepted
- Date: 2026-07-14
- Supersedes: [ADR 0007](0007-external-app-integration-contract.md)

## Context

MedTracker serves first-party applications, local automation, hosted MCP
clients, and external healthcare applications. These clients require different
payloads, compatibility guarantees, credentials, and consent boundaries.

ADR 0007 established separate product and FHIR surfaces, but it was written
across the delivery of SMART on FHIR. It consequently describes SMART support
as both deferred and implemented. The shipped architecture now includes a
registered-client SMART App Launch flow, so one current decision must replace
that contradictory record.

## Decision

MedTracker keeps separate integration contracts by audience. No single API is
the canonical interface for every client.

| Audience | Contract | Surface |
| --- | --- | --- |
| First-party mobile applications | MedTracker product API with portable IDs, household routes, session exchange, sync, and export/import | `/api/v1` |
| First-party CLI and local automation | MedTracker product API using API sessions or API app tokens | `/api/v1` |
| Hosted MCP clients | Read-only Streamable HTTP MCP using MedTracker bearer credentials | `/mcp` |
| Registered third-party healthcare applications | SMART App Launch 2.x standalone authorization and FHIR R4 read/search resources | `/authorize`, `/token`, `/revoke`, and `/api/fhir/R4` |
| Browser and operator workflows | Rails web UI and explicitly supported administration endpoints | Web routes and `/api/v1` admin routes |

First-party clients use `/api/v1` for MedTracker-specific workflows. FHIR is
not the product synchronization API and does not replace sync batches,
tombstones, encrypted portable data, household administration, or capability
negotiation.

External healthcare applications use FHIR R4. They must not depend on
MedTracker product payloads unless MedTracker explicitly supports them as a
first-party or trusted product client.

## First-party authorization

MedTracker owns the authorization context for `/api/v1` and `/mcp`.

- API sessions are minted after a valid MedTracker password login or external
  OIDC identity exchange.
- API app tokens are created from an authenticated MedTracker profile session.
- Each credential is bound to an account, household, membership, and membership
  permissions version.
- Person access is evaluated from active grants and Pundit policy scopes.
- Locked accounts, inactive users, revoked memberships, stale permissions, and
  cross-household requests fail closed.
- The hosted MCP surface uses the same bearer boundary and remains read-only.

The external identity provider owns primary authentication, MFA, recovery, and
passkeys as recorded in ADR 0005. MedTracker still owns its API session,
household, grant, revocation, and audit behavior.

## SMART on FHIR authorization

MedTracker supports SMART App Launch 2.x standalone launch for registered
third-party healthcare applications.

The discovery document is published at
`/api/fhir/R4/.well-known/smart-configuration`. The FHIR R4 capability statement
advertises the authorization, token, and revocation endpoints.

Authorization uses the code flow. Public clients use PKCE with `S256`.
Confidential clients also authenticate with their registered client secret.
Redirect URIs are registered in advance, must use HTTPS, and are compared
exactly. Authorization codes are short-lived, single-use, and bound to the
client, redirect URI, and verifier.

The consent decision binds the OAuth grant to:

- the registered application;
- the signed-in account;
- one active household membership and its permissions version;
- one person context; and
- the approved SMART scopes.

FHIR reads must pass both the SMART resource scope and the MedTracker policy
scope. A scope cannot expand household or person access. Changed or revoked
memberships, locked accounts, inactive users, expired grants, and revoked grants
fail closed before FHIR data is returned.

Supported scopes are read-only SMART v2 patient and user scopes, including
`patient/*.rs`, `user/*.rs`, and supported resource-specific forms. Access
tokens expire after 15 minutes. Refresh tokens expire after 30 days and rotate
on use. Revocation invalidates the complete grant.

Authorization codes, access tokens, and refresh tokens are never stored in
plaintext. Token digests and PHI-free consent and revocation evidence are stored
without FHIR payloads or raw credentials.

## Deliberate boundaries

Only the shipped contract is advertised:

- standalone launch is supported; EHR launch is not supported;
- read scopes are supported; FHIR write scopes are not supported;
- Dynamic client registration is not supported;
- bulk-data export is not part of the SMART contract;
- OpenID Connect identity scopes are not part of the SMART contract; and
- `/api/v1` credentials do not become SMART grants, and SMART grants do not
  authorize `/api/v1` product operations.

Registered-client onboarding is therefore an explicit trust and operational
process. Expanding any boundary above requires a new decision or an amendment
that covers consent, tenant isolation, revocation, audit, and compatibility.

## Consequences

### Positive

- Client teams have an unambiguous endpoint and credential choice.
- Product sync can evolve without forcing MedTracker concepts into FHIR.
- Third-party healthcare access uses standard discovery, consent, scopes, and
  FHIR payloads.
- SMART scope checks cannot bypass MedTracker household and person policy.
- Discovery metadata can remain truthful about the supported launch mode.

### Negative

- MedTracker maintains separate product, MCP, and FHIR documentation surfaces.
- Clients that need both product workflows and FHIR reads require separate
  credentials and an explicit onboarding decision.
- Registered SMART applications require operational client management until a
  deliberately designed registration workflow exists.

## Related documents

- [External Identity Provider](0005-external-identity-provider.md)
- [Bounded Context Map](0009-bounded-context-map.md)
- [SMART on FHIR](../api/smart-on-fhir.md)
- [SMART on FHIR security review](../security/smart-on-fhir-security-review.md)
- [MCP integration](../mcp.md)
- [MedTracker OpenAPI contract](../api/openapi.v1.yaml)
