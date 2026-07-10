# SMART on FHIR

MedTracker exposes a SMART App Launch 2.x standalone authorization flow for
registered third-party healthcare applications. It is separate from the
first-party `/api/v1` API, CLI credentials, and hosted MCP access.

## Discovery and registration

Read the server contract from
`/api/fhir/R4/.well-known/smart-configuration`. A registered client has an exact
HTTPS redirect URI, a client identifier, approved SMART scopes, and optionally
a confidential client secret. Redirect URIs are compared exactly; wildcard and
HTTP callback URIs are rejected.

## Authorization

Start at `/authorize` with `response_type=code`, the registered `client_id` and
`redirect_uri`, requested scopes, state, and an `S256` PKCE challenge. The
consent page shows the client, requested scopes, household, and person. The
grant records the membership permissions version, so membership revocation or a
permission change invalidates existing access.

Exchange the code at `/token` with its PKCE verifier. Access tokens last 15
minutes. Refresh tokens last 30 days and rotate whenever they are used. Revoke a
grant through `/revoke`.

## Scope and policy enforcement

Supported read scopes use SMART v2 syntax, including `patient/*.rs` and
resource-specific forms such as `patient/MedicationRequest.rs`. `user/*.rs` is
also available. A SMART scope never replaces MedTracker authorization: FHIR
queries remain restricted by the bound household membership, person grants,
account state, and Pundit policy scopes.

## Security and audit behavior

Access and refresh tokens are stored only as SHA-256 digests. Consent and
revocation audit events identify the client, account, membership, person, and
scopes without recording authorization codes, token material, or FHIR payloads.
Exact redirect matching, single-use authorization codes, PKCE, rotating refresh
tokens, membership-version checks, and account lockout checks limit replay and
scope escalation.
