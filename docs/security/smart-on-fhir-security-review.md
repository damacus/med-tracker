# SMART on FHIR security review

This review covers third-party SMART authorization only. First-party `/api/v1`,
CLI, and MCP credentials retain their existing authentication paths.

## Controls

- Tokens: access and refresh credentials are random, short-lived, and stored as
  SHA-256 digests. Confidential client secrets use password hashes. Raw token
  values exist only in authorization responses.
- Redirects: every registered callback is an exact HTTPS URI. Authorization
  rejects any URI that is not in the client registration.
- PKCE and replay: public clients must use `S256`. Authorization codes are
  single-use, bound to the redirect URI and verifier, and expire before access
  tokens are issued.
- Scope escalation: requested scopes must be a subset of the registered client
  scopes. FHIR controllers check the SMART resource scope and then apply the
  existing household and person policy scope.
- Tenant context: grants bind account, household membership, person, and the
  membership permissions version. Inactive or changed memberships, locked
  accounts, inactive users, expired grants, and revoked grants fail closed.
- Refresh and revocation: refresh tokens rotate on use and expire after 30
  days. Revocation invalidates the complete grant, including its access and
  refresh credentials.
- Audit and logs: consent and revocation events record client, account,
  membership, person, and scopes. Codes, token material, and FHIR payloads are
  excluded and the shared audit redactor remains active.

## Residual boundaries

Only standalone launch is advertised. EHR launch context, dynamic client
registration, write scopes, bulk data, and OpenID Connect identity scopes are
not implemented and do not appear in discovery metadata. OAuth grant lookup
must occur before tenant context is known, so the grant table is not protected
by household RLS; every accepted grant is immediately checked against its
bound active membership and permissions version before tenant context is set.
