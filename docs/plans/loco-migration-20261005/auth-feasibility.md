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

### Bounded architecture clarification during implementation

A read-only Astra source check inspected oxide-auth master
`0e3ef86d924aa5546dc3275d1d7269ea63edfaf1`, dated 31 January 2026.
This establishes some maintenance activity, correcting the earlier unconfirmed
maintenance statement; it does not establish a security-support commitment.
Released docs identify 0.6.1. Reconcile these master findings with the actual
locked release before relying on them. No dependencies were installed or tests run.

- Async access-token flow supports body credentials; the high-level async refresh
  wrapper only parses Basic headers. Its lower-level request adapter can invoke
  the library-owned refresh engine without copying grant transitions.
- Registrar validation receives client ID and optional secret but no authentication
  method. Enforce each client's permitted method with immutable request context.
  The refresh request trait also lacks client-ID access; bind a supplied public
  client's ID explicitly rather than silently ignoring it.
- The refresh engine proposes a one-hour access expiry. The durable issuer must
  enforce MedTracker's configured access, refresh and inactivity policies instead.
- SMART's top-level patient field needs response enrichment from verified stored
  grant context; standard token serialisation alone does not supply it.
- Authorizer/Issuer storage seams permit atomic code consumption, refresh rotation
  and digest compatibility, but only database/interoperability tests can prove them.
- No runnable RFC7009 HTTP orchestration was found in the inspected endpoint
  modules. A narrowly composed revocation adapter remains an explicit protocol
  responsibility requiring necessity documentation, negative tests and review.

Recommendation: run I1's bounded interoperability proof with oxide-auth as a
conditional in-process candidate. Do not select it as complete or assume Hydra
is required. Escalate only if that proof requires an incompatible transition,
external provider or unsupported custom protocol ownership. The source inspection
does not accept identity migration or change the existing security constraint.

Exact inspected source:

- [Async refresh wrapper](https://github.com/197g/oxide-auth/blob/0e3ef86d924aa5546dc3275d1d7269ea63edfaf1/oxide-auth-async/src/endpoint/refresh.rs)
- [Async grant engine](https://github.com/197g/oxide-auth/blob/0e3ef86d924aa5546dc3275d1d7269ea63edfaf1/oxide-auth-async/src/code_grant.rs)
- [Registrar interface](https://github.com/197g/oxide-auth/blob/0e3ef86d924aa5546dc3275d1d7269ea63edfaf1/oxide-auth/src/primitives/registrar.rs)
- [Refresh engine](https://github.com/197g/oxide-auth/blob/0e3ef86d924aa5546dc3275d1d7269ea63edfaf1/oxide-auth/src/code_grant/refresh.rs)
- [Token response](https://github.com/197g/oxide-auth/blob/0e3ef86d924aa5546dc3275d1d7269ea63edfaf1/oxide-auth/src/code_grant/accesstoken.rs)

### Identity proof acceptance

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
