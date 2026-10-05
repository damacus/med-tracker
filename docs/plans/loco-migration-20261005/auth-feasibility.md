# Authorization-server feasibility findings

Read-only Scout research on 5 October 2026. No runtime checks or implementation.
Actual Rails uses Rodauth OAuth server, not merely OAuth client authentication.
SMART discovery advertises authorize/token/revoke, S256, authorization-code and
refresh grants and public-none/confidential Basic/secret-post authentication.
Integration grants additionally bind membership, person and permissions version.

## Established in-process candidate

Oxide-auth exposes AuthorizationFlow, AccessTokenFlow, RefreshFlow and ResourceFlow.
Its PKCE addon must be applied to authorization and token flows with persisted
private extension data. Authorizer::extract invalidates a code; the durable adapter
must atomically consume stored grants. Issuer supports recover_refresh/refresh;
its TokenMap example rotates tokens, but production needs durable transactional
storage and existing expiry policies. Registrar adapts registered clients,
redirects, scopes and client-secret checks. These are library-owned grant engines,
unlike existing Rust handlers that use only the PKCE helper.

The concrete open gaps are library-owned RFC7009 revocation, per-client permitted
authentication-method enforcement and exact async refresh/secret-post support.
TokenMap::revoke is a storage method, not a standards-complete HTTP revocation flow.
Do not add bespoke protocol handlers to fill these gaps merely to keep a monolith.
Absence of a named revocation flow alone does not rule out framework glue:
oxide supplies HTTP parameter/header interfaces, credential parsing, Registrar
checks and grant recovery with client identity. No maintained implementation was
demonstrated that composes these into the complete revocation endpoint. A narrow
adapter would therefore need an explicit dependency limitation and necessity
record, minimal implementation, standards/interoperability and negative security
tests, and independent review before adoption. It must not retain the existing
custom grant state machine. A wrong token hint must permit lookup fallback;
unknown-token success must not skip client authentication or token ownership.
Recent core maintenance was not confirmed; the Axum 0.8 adapter dates to January
2025. Context7 returned specification text rather than implementation evidence.

OAuth-as documents broader protocol ownership, but its first releases date to
August 2026; API breadth is not established maturity. Do not adopt it as though
that question were answered. Rauthy is a separate Rust identity provider with an
independent security audit; it is an external architecture candidate, not a proven
SMART/patient-context or stored-credential migration solution.

## Concrete external fallback

Hydra delegates login and consent to the application, allowing Loco to retain
MedTracker accounts and permission decisions. Its maintained grant engine covers
PKCE, confidential/public authentication, refresh rotation and revocation. Current
research found a documented August 2026 security release. This remains a fallback,
not a selected architecture.

Existing native client IDs and redirects can be registered, but Rodauth token and
grant hashes are not imported as valid Hydra tokens. Reauthorization or explicit
coexistence would be needed. Confidential secret-hash import was not established;
client secret rotation and single-method registration may be required. Consent
session claims are not evidence of SMART's top-level token-response patient field:
that extension and native inactivity expiry still require proof. Rauthy owns more
identity/session infrastructure and also lacks demonstrated SMART response parity.

## Required evidence before selection

Prove native S256 and confidential Basic/post/public-none; reject disallowed or
mixed credentials, duplicate parameters, redirect mismatch and bad verifiers.
Test expired/replayed/concurrently redeemed codes; refresh narrowing/expiry/reuse/
rotation races; RFC7009 form requests and unknown-token success with wrong-client
denial; actual token invalidation; SMART patient context, withdrawn consent, stale
permissions and transactional audit rollback.

The authorization-server decision remains open. A separate provider changes
deployment, issuer/token storage, client registration and consent integration.
An external provider must be selected explicitly before those changes are made.
Independent schema baseline work and foundation acceptance can continue.

Primary sources:

- <https://docs.rs/oxide-auth/latest/oxide_auth/endpoint/index.html>
- <https://docs.rs/oxide-auth/latest/oxide_auth/code_grant/extensions/struct.Pkce.html>
- <https://docs.rs/oxide-auth/latest/oxide_auth/primitives/authorizer/trait.Authorizer.html>
- <https://docs.rs/oxide-auth/latest/oxide_auth/primitives/issuer/trait.Issuer.html>
- <https://raw.githubusercontent.com/197g/oxide-auth/master/oxide-auth-async/src/primitives.rs>
- <https://docs.rs/crate/oxide-auth-axum/latest>
- <https://docs.rs/oauth-as/latest/oauth_as/>
- <https://github.com/sebadob/rauthy>
- <https://docs.rs/oxide-auth/latest/oxide_auth/endpoint/trait.WebRequest.html>
- <https://www.rfc-editor.org/info/rfc7009/>
- <https://www.ory.com/docs/oauth2-oidc/custom-login-consent/flow>
- <https://changelog.ory.com/announcements/ory-hydra-v26-3-11-released>
- <https://raw.githubusercontent.com/ory/hydra/master/client/client.go>
- <https://github.com/ory/hydra/issues/2847>
