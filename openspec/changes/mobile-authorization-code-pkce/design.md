# Design

## Context

See proposal.md for motivation. Baseline: main `62db0c58`, including merged
PR #2232. The parent change `unify-mobile-login-with-rodauth` remains unarchived;
its verification record distinguishes completed local checks from missing live
provider and boundary evidence. Its unchecked tasks are not proof of missing code.

`spec/requests/mobile_oauth_authorization_spec.rb` already covers successful
redemption, local browser login, enrolled OTP, wrong/missing verifier, callback
mismatch, sequential replay, refresh rotation and household role changes.
Extend these cases, not a replacement suite. `RodauthMobileOauth` wraps grant
eligibility and refresh; the OAuth library owns PKCE and code consumption.

## Goals / Non-Goals

**Goals:** close observable failure-path and deployed-login evidence gaps with
small behaviour fixes only when reproduced; retain the parent's account-level
and per-request household authorisation model.

**Non-Goals:** another protocol or session store; broad policy changes;
reactivating the deleted exchange; implementing all unchecked parent tasks as
though none of their code existed. Do not couple this security acceptance work
to an unimplemented offline replay engine or unrelated iOS delivery.

## Decisions

### Keep the two authentication boundaries explicit

The phone authorises against MedTracker and redeems its code at MedTracker.
Rodauth can authenticate the browser locally or delegate to ZITADEL through
the existing OIDC integration. Those are different client registrations and
callbacks. Native ZITADEL client IDs and `OIDC_MOBILE_CLIENT_ID` from the retired
exchange are not evidence that current Rodauth discovery is configured correctly.
Use live discovery and registered MedTracker mobile applications when checking
the deployed flow. Verify upstream issuer/audience/expiry/nonce/signature at the
existing OIDC boundary; do not add bespoke token validation to the phone bridge.

The library's [PKCE documentation](https://honeyryderchuck.gitlab.io/rodauth-oauth/wiki/PKCE)
places challenge handling at authorization and verifier checking at token
redemption. Confirm configuration against the locked gem during implementation;
online examples alone are not evidence for the installed version.

Rejected: adding a verifier comparison to the deleted ID-token exchange, or
deploying PR #2222. Both target the superseded architecture.

### Test public failures and database outcomes

Use real HTTP requests, persisted grants and the installed OAuth implementation.
For code races, use independent request sessions and database connections with
explicit synchronisation, not sleeps or mocked successful redemption. Ensure
test fixtures are committed and visible to both connections. Assert at most one
successful issuance and no second credential lineage, then exercise returned
credentials at the API boundary. A replay policy may revoke the winning token;
the invariant is never two independently usable sessions.

Inject persistence failure at the grant-write boundary inside the transaction.
Assert no success response, no usable token/refresh state and no successful
issuance audit survive rollback. For a confirmed rollback, the original code can
be retried before expiry and may succeed once. For a lost response after commit,
retry must not mint another token pair: restart browser authorization rather than
cache or replay plaintext credentials. Keep refresh rotation library-managed.

Rejected: regex tests of Ruby/YAML and a custom PKCE implementation. A regression
test already passing requires no manufactured production change.

### Reuse authorisation and private audit boundaries

Authenticate account grants before resolving each request's membership in
`Api::V1::BaseController`. Test concurrent household requests and membership loss
with independently constructed HTTP sessions. Check record visibility, denied
writes and actual audit membership, not just a status code. Restricted SMART/FHIR
and app-token credentials retain their existing limits. A failed transaction must
not leave a domain write, success audit or idempotency success result.

Test redaction using synthetic sentinel values in codes, verifiers and tokens.
Check request, provider-error, trace and audit paths used by the changed flow;
never collect real credential bodies to prove redaction. Keep upstream failures
generic and preserve existing role/person and account-lockout checks.

### Separate repeatable tests from live acceptance

Use deterministic OIDC fixtures for malformed identity responses. Separately
exercise a real configured provider with a test account through Android's system
browser: first login, eligible SSO reuse, configured passkey/MFA, cancellation,
refresh, sign-out and authenticated household/dashboard reads. Local-only login
must continue to work. Provider policy can require interaction; do not declare
SSO broken solely because it correctly requests a factor.

Record server commit/image digest, Android build, environment, public callback
and client configuration, test time and redacted outcome. Missing device or
provider access is an explicit blocked check, not a pass. Deployment thread
`019f5a48-2fdc-7c32-a06a-923848f5efcb` owns the separately authorised rollout.

### Reconcile rather than duplicate parent acceptance

Map results to parent tasks 2.2/2.3/2.5/2.6, 3.3/3.4/3.5, 4.1/4.2 and
5.2/5.3/6.1/6.3. Mark only demonstrated portions complete. Keep unrelated parent
tasks open. Update ADR 0005 and design/setup descriptions that still claim direct
provider mobile exchange. Do not close #1889 until its accepted replacement flow,
security cases and live delivery are supported by current evidence.

## Risks / Trade-offs

- Race tests can pass without overlapping requests → synchronise at the database
  boundary and prove two independent connections entered the critical path.
- Shared OAuth changes can broaden integration grants → retain SMART/FHIR
  regression checks and credential-kind invariants.
- Provider mocks can hide callback or configuration errors → require real browser
  acceptance separately, using no patient data.
- Deployment may advance while this proposal is implemented → pin evidence to
  server and client versions, then retest the final deployed pair.

## Migration Plan

This proposal requires no schema migration or live configuration changes.
Ship any reproduced fixes through normal review after focused then full relevant
checks. Obtain live evidence from the deployment owner; this plan does not grant
new deployment authority. Rehearse rollback on a representative environment
without production data: select a schema-compatible release that retains the
Rodauth flow, or disable new issuance while correcting the fault. Do not roll back
blindly to a release that interprets account grants as household-bound sessions.
Revoke affected test credentials and verify restricted integrations still work.

## Open Questions

- Which test device and provider test account can exercise configured passkey
  login? Resolve before live acceptance, without changing authentication policy.
- Which schema-compatible release will the deployment owner use for rollback
  rehearsal? Record its digest before the rehearsal.
