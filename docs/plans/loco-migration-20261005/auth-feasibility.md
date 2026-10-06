# Authorization-server feasibility findings

## Full account lifecycle research — 6 October 2026

The owner prioritises a feature-complete maintained Rodauth replacement embedded
inside MedTracker, with its own daisyUI pages and database. Rauthy and all separate
identity services are rejected. Better Auth RS is selected,
including OrganizationPlugin for households and multiple memberships. Clearing
existing passwords and MFA enrolments is
accepted, provided secure reset/onboarding works before access. The existing root
registrations have no password-reset route; eleven intended reset contracts remain
private and unexecuted. New custom lifecycle implementation is paused. Integrate
the selected library through normal account and household journeys, without a
separate pre-implementation proof programme. No dependency has been installed,
deployed or exercised against MedTracker yet.

| Option | Verified fit and limitation |
| --- | --- |
| [Rauthy v0.37.0](https://github.com/sebadob/rauthy/releases/tag/v0.37.0) | Separate Rust OIDC identity service with account/admin UI, activation/reset mail, passkeys and attack controls. Version 0.37 adds opt-in email OTP; authenticator-app TOTP is still unimplemented. |
| [rs-auth](https://github.com/rs-auth/rs-auth) | Embedded signup, verification, reset, sessions and social login. Documented feature set does not establish required TOTP/passkey/full lifecycle coverage or independent audit evidence. |
| [moso-auth](https://docs.rs/crate/moso-auth/latest) | Advertises lifecycle/MFA/passkeys, but published 0.0.1 dates to 19 August 2026 and depends on its own framework/ORM graph. Feature claims are not maturity or integration proof. |
| [Better Auth RS](https://github.com/better-auth-rs/better-auth-rs) | Current master advertises Axum and application-owned SeaORM entities, but 1.0.0-alpha.3 APIs/schema may change and current DX examples are not a published release. Upstream TypeScript Better Auth maturity cannot be attributed to this Rust implementation. |

The following Rauthy findings are retained as research history, not an adoption
recommendation: the separate-service architecture is ruled out. Its
[documentation](https://sebadob.github.io/rauthy/) reports an independent
audit with findings addressed in 0.32.1. The report itself and latest-release coverage
have not been verified. Its [official Dockerfile](https://raw.githubusercontent.com/sebadob/rauthy/main/Dockerfile)
uses distroless cc-debian12 non-root, not scratch. MedTracker's own scratch server/
worker requirement remains; a separate IdP image boundary needs an explicit decision.
The [current OTP config](https://raw.githubusercontent.com/sebadob/rauthy/main/config.toml)
states only email OTP is implemented. Resetting old MFA does not resolve whether
future authenticator-app TOTP is required.

The rejected external design would let Rauthy own credential lifecycle and its
account UI. A maintained OIDC
client supplies verified issuer/subject; explicit unique mappings preserve local
account IDs. Never link care accounts dynamically from submitted/unverified email.
Cedar, current local grants and PostgreSQL RLS own clinical access. Household
administrators do not become identity-service administrators. Creation/deletion
across two systems needs controlled retry/reconciliation rather than a claimed
single SQL transaction.

[Forced logout](https://sebadob.github.io/rauthy/work/logout.html) removes sessions
and refresh tokens and sends backchannel notifications. Local JWT signature/expiry
checks alone do not establish immediate invalidation. That documentation describes
client-side backchannel helpers as future work; verify the actual maintained client
or introspection capability before integration, rather than writing a new protocol.

Normal account and household journey tests must provision an explicitly mapped existing account, receive
activation/reset mail, sign in to that exact account, deny cross-household access,
invalidate the MedTracker session on logout/disable, restart without losing mapping,
and retry closure safely. No production credentials or account mutations. Outstanding
feature decisions include exact password lockout/email unlock behaviour and any
optional absolute session-age limit. Existing published source and
all full migration requirements remain until this replacement is proved.

Better Auth RS research must use a reviewed immutable revision rather than assume
master documentation matches released `1.0.0-alpha.3`. The inspected master revision
is `9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281`. Prove passwordless migrated-account
reset, mail callbacks, immediate session withdrawal with cookie caching disabled,
transactional account/person/household/audit writes, and both scratch architectures.
Organization membership may own household identity and roles, but it must preserve
scoped suspension, person grants and Cedar without two mutable membership authorities.
An active-organisation session selector does not authorise a household URL. Protect
multi-tab access and live membership changes. Plugin admin routes and deletion must
not bypass Cedar, retention holds, last-owner rules or clinical history preservation.

Rails production verification grace is zero; no seven-day production grace needs
reproducing. Password-only timed lockout and emailed unlock are not proved by client
rate limiting. Passkeys must not count as MFA without proved required user
verification. The owner requires passkeys as passwordless primary login and TOTP
as the separate MFA capability, and permits omitting email OTP. Recovery codes
are required regardless of login method, including recovery after loss of a sole
passkey, without mandatory TOTP enrolment. TwoFactorPlugin's documented backup-code
flow is not proof of that independent recovery capability. Investigate a maintained
implementation or upstream extension; do not substitute reset email or a custom
token protocol. Email ownership verification does not clear a failed-login lockout;
the owner's verification-documentation link is not approval to omit emailed unlock.
Plugin OAuth client sign-in is not evidence of a replacement OAuth
authorisation server; deferred native/SMART work remains in the full migration goal.

### Embedded integration findings at the pinned revision

Read-only source inspection establishes one usable transaction seam and several
gaps to exercise and resolve during the selected library's normal implementation:

| Application requirement | Actual maintained-library seam or limitation |
| --- | --- |
| Atomic signup and clinical-account provisioning | [Email/password signup](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/api/src/plugins/email_password.rs#L593) creates user, credential account and optional session through `TransactionStore`. [SeaORM hook context](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/seaorm/src/hooks.rs#L27) exposes that transaction, allowing application person/household/membership/audit writes to join it and propagate failure. Prove forced audit failure rolls every write back. |
| Reset a migrated account with no password | [Reset handlers](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/api/src/plugins/password_management/handlers.rs#L108) create the missing credential account. Token consumption, password write and session withdrawal are separate calls; `on_password_reset` errors are logged and ignored. The [transaction interface](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/core/src/store/mod.rs#L30) exposes only user/account/session creation. A callback cannot make the entire reset atomic. Prove the write-failure case and establish a maintained transaction extension rather than copying reset logic. |
| OrganizationPlugin owns household memberships | [Custom schema](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/core/src/schema.rs#L6) covers User, Session, Account and Verification; that limits bundled SeaORM entity substitution, not the public storage interface. [Public OrganizationStore/MemberStore](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/core/src/store/mod.rs#L142) can map canonical existing household/membership tables. IDs are strings without a UUID constraint, allowing checked conversion of existing numeric IDs. Prove this adapter round-trip and live withdrawal without synchronised duplicate authorities. |
| Household creation and invitation acceptance | [Organization creation](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/api/src/plugins/organization/handlers/org.rs#L71) writes organization and member separately; invitation acceptance similarly separates member creation and invitation status. Account/member/person grants and audits need one transaction or a demonstrated maintained extension. |
| Tenant RLS and safe account closure | Ordinary SeaORM store operations use the pooled connection rather than a supplied tenant transaction. [User deletion](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/seaorm/src/store/users.rs#L162) uses a hook context without a transaction and separates API-key/user deletion. An after-delete audit cannot undo it. Establish tenant context in the actual transaction and guard closure with clinical retention/last-owner rules. |

These findings do not reject OrganizationPlugin, approve feature drops or authorise
a competing membership implementation. They identify the exact storage and
transaction adapters required for the selected direction. Published alpha APIs
may differ from these master sources; any adoption must pin the reviewed revision.

Independent recovery remains a demonstrated gap at this pin. [Code generation](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/api/src/plugins/two_factor/actions.rs#L352)
requires an authenticated session, enabled two-factor state, a stored credential
password and a TwoFactor row. [Redemption](https://github.com/better-auth-rs/better-auth-rs/blob/9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281/crates/api/src/plugins/two_factor/mod.rs#L612)
requires an existing session or a signed pending MFA challenge. There is no public
configuration/helper for a passwordless, TOTP-free user to issue recovery codes or
start recovery after losing their sole passkey. Prove that case explicitly and
establish a maintained upstream extension; do not drop the requirement or introduce
an application-owned recovery protocol.

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
