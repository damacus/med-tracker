# Identity and protocol compatibility Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve existing sign-in, security settings and authorised clients using maintained security implementations.
**Architecture:** Library-owned protocol flows validate credentials; Loco applies current household permissions
through persistence interfaces. Select the OAuth server implementation from evidence before writing its adapter.
**Tech Stack:** Loco 1.2.0; maintained bcrypt, session, CSRF, TOTP, WebAuthn and OAuth server capabilities.
**Spec:** [implementation-spec.md](../implementation-spec.md), [auth-feasibility.md](../auth-feasibility.md).

## Global constraints

P3 supplies tenant persistence. No custom grant engine or cryptographic primitive.
External provider selection or incompatible credential/client transitions require an explicit decision.
Synthetic credential fixtures only. Root planning authority owns the decision record; writer owns adapters/tests.

### Approved passkey transition — 6 October 2026

The user approved dropping credential algorithms unsupported by the maintained Rust
library, even when affected users must reauthenticate and replace their passkeys.
Do not pursue PS256 compatibility, add a provider or implement a custom verifier.
Preserve existing credential rows for Rails rollback. Supported retained credentials
must still work; an unsupported credential must not block another supported key.
Security settings identify keys that need replacement. Use existing verified
password/recovery and password-confirmed replacement flows without silently
bypassing MFA. Test clean unsupported rejection, mixed-key accounts, the replacement
message and removal/re-enrolment. This approved transition replaces the requirement
to authenticate every historical passkey algorithm in Loco.

Counting affected production accounts is not a gate. Saved pre-cutover state keeps
the Rails rollback credentials. Existing sessions and tokens will be invalidated;
historical continuity is unnecessary. Test the invalidation mechanism on synthetic
credentials and retain fresh authentication, account security and MFA checks.
FHIR/SMART integration is deferred from the first production gate; it remains in
the full migration scope. Core native/public API sign-in stays in the first release.

## Review focus

Existing stored formats must work (I1/I2). Duplicate, mixed and disallowed client credentials fail (I1/I3).
Concurrent code/refresh redemption cannot mint duplicate valid grants (I3). Wrong revocation hints cannot
prevent valid token lookup (I3). Revoked sessions and cross-origin mutations fail immediately (I2/I3).

### I1: Resolve protocol and stored-format compatibility with an executable decision

**Files:** Create `docs/plans/loco-migration-20261005/identity-decision.md`,
`tests/identity_compatibility.rs`, `tests/fixtures/identity/manifest.json`;
read `rails/app/misc/rodauth_main.rb`, existing Rust compatibility tests and retained source formats.
**Interfaces:** Produce a chosen dependency/version, stored-format inventory and signed-off compatibility strategy.
No later identity task starts with an undecided protocol implementation.

- [ ] Define executable conformance cases for native S256; public-none/confidential Basic/secret-post;
  refresh rotation; RFC7009 form revocation; SMART patient response; old passwords/TOTP,
  supported passkeys, and rejection of invalidated historical sessions/tokens.
  Assertions include `mixed_credentials_accepted == false`, `wrong_redirect_accepted == false`,
  `second_code_redemption_succeeds == false`, `old_password_authenticates == true`.
- [ ] Run `rtk task slice:test TARGET=identity_compatibility`; record unsupported cases against the candidate,
  rather than treating PKCE helper availability as a complete OAuth server.
- [ ] Conduct one bounded library proof using the existing feasibility findings and current primary docs.
  Record the exact remaining adapter surface, maintenance evidence and necessity for any protocol glue.
  If external deployment or reauthorisation is necessary, produce its concrete compatibility decision for
  the user. Escalate unresolved security architecture to Astra; do not open a generic follow-up issue.
- [ ] Run the conformance proof; require every required method/format to pass or have an explicitly approved
  transition before integration. Reconcile all environment configuration, including password pepper.
- [ ] Commit the decision/proof separately as `docs(identity): record verified authentication architecture`.

### I2: Preserve browser authentication, MFA/passkeys and security middleware

**Files:** Extend `src/models/identity/`, `src/controllers/auth.rs`,
`src/controllers/auth/`, `src/controllers/signup.rs` and the existing browser
specs/fixtures under `tests/browser/`. Register routes in `src/app.rs`; add
dependency/configuration only from I1's selected architecture. Use the existing
browser runner and identity test binaries; do not introduce a second browser
runner or a parallel identity module.
**Interfaces:** `async fn identity::authenticate(db: &DatabaseConnection, credential: &VerifiedCredential)
-> Result<Actor, OperationError>`;
`VerifiedCredential` is a wrapper around the chosen library's validated result, never caller-supplied identity.
Produces validated actor and preserved account/session policies, not household authorisation.

- [ ] Test existing password fixtures, long-password limits, expired/revoked sessions, MFA recovery,
  passkey origin/RP mismatch and counter policy. Test CSRF rejection, CSP/cookie headers and rate limits.
  Assert `forged_actor_accepted == false`, `revoked_session_status == 401`,
  `cross_origin_mutation_committed == false`; use the contract's established status for CSRF rejection.
- [ ] Use the existing filtered browser task to demonstrate missing behaviour,
  then run all affected identity journeys through the same runner.
- [ ] Integrate the selected maintained implementations and original stored-format compatibility.
  Preserve expiry/revocation settings; do not swap existing account storage for generated sample users.
- [ ] Finish invited signup and configurable open registration, including first-household
  bootstrap. Preserve `INVITE_ONLY` precedence over the stored setting and the
  missing-setting initialisation from whether an active owner exists. A stored
  false setting stays open after bootstrap until changed. Verification must
  use the retained one-use key format, real queued email delivery and status
  transition before clinical access. Test atomic rollback, duplicate email,
  closed registration and owner/self-care effects with the restricted database role.
- [ ] Run identity tests and root CI; exercise real login, MFA/passkey and logout requests on an owned fixture.
- [ ] Review and commit `feat(identity): preserve account security in Loco`.

### I3: Expose the complete OAuth/SMART authorisation server contract

**Files:** Extend `src/controllers/oauth_server/`, `src/models/identity/oauth.rs`,
`tests/oauth_server.rs` and `tests/oauth_server/`;
modify dependency/config files and `src/app.rs`; read `rust/contract-tests/tests/oauth.rs`
and `smart_fhir.rs`. The selected library owns grant state transitions.
**Interfaces:** `oauth::routes() -> Routes`; discovery/authorize/token/revoke endpoints retain the
authoritative contracts, client bindings, patient context, current membership and consent checks.

Execute four independently testable tasks with the cycle below: `authorisation_code`,
`refresh_rotation`, `token_revocation` and `smart_consent` filters in `oauth_server`.
The selected library and I1 decision are fixed inputs for each; do not redesign them
between tasks. Code redemption races belong to the first task, refresh races to the second,
hint/client ownership to the third, and withdrawn/stale consent to the fourth.

- [ ] Test concurrent code redemption and refresh races; scope narrowing; token expiry/reuse;
  mixed credentials; wrong token hint; authenticated unknown-token revocation; wrong-client denial;
  withdrawn consent and stale permission version. Assert `valid_grants_after_code_race == 1`,
  `revoked_token_can_read == false`, `unknown_token_revocation_status == 200` after valid client auth.
- [ ] Run `rtk task slice:test TARGET=oauth_server`; record failing standards and compatibility cases.
- [ ] Adapt the I1 library with durable transactional storage and exact client authentication enforcement.
  Keep any justified glue within the reviewed decision; no revived legacy custom grant engine.
- [ ] Run I1 conformance again, I3 tests, root CI and native/SMART interoperability checks.
- [ ] Obtain security-focused Devin review, integrate and publish `feat(identity): preserve OAuth and SMART grants`.

**Done:** Stored formats, browser security, native clients and SMART flows pass negative/concurrent
tests with the chosen maintained implementation. Any unavoidable transition is explicitly approved and tested.
