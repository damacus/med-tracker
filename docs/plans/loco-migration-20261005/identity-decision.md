# MedTracker authentication delivery contract

Approved 7 October 2026. Supersedes the passwordless-only draft and proposed
library patches. Deliver one auth-only PR; wider migration remains paused.
Preserve account IDs, clinical records, household permissions and Rails rollback
data. Native UI, deployment and merging are excluded.

## Dependencies and integration

Use unmodified Better Auth RS through Cargo at official upstream revision
`9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281`, with committed Cargo.lock. No fork,
vendored source, local filesystem dependency or library patch. Configure password,
password management, email verification, passkeys, TOTP, session/account/user
management, organisations, OAuth and API-key capabilities. Exclude impersonation
and unrestricted admin endpoints. Supported hooks and transaction-bound storage
integrate canonical SeaORM accounts, memberships and audit records. Remove draft
patch-only interfaces. Use `openidconnect = "=4.0.1"` for ZITADEL verified
fresh-authentication proofs; retain the existing native OAuth grant engine.

The pinned Better Auth passkey plugin hard-codes discouraged user verification
and discards the current authentication UV result. Use maintained webauthn-rs
0.5.5 high-level passkey ceremonies, which enforce Required UV, behind a supported
application AuthPlugin and Better Auth storage/session APIs. Keep upstream passkey
handlers unregistered. Persist challenge state server-side, bind registration to
its enrolment session and consume state atomically; reject missing UV and replay.

Pinned Better Auth applies TOTP account lockout only to pending sign-in challenges,
not verification from an existing session. Pending security operations therefore
use its supported factor-store failure/reset methods and public lockout defaults
around the same upstream TOTP verifier. Read and bind the pending operation before
proof; consume it only after successful proof. Only the exact supported invalid-code
and exhausted-challenge outcomes, or the adapter's temporary factor lock, permit
failure-accounting state to commit. Session issuance, credential changes and failed
audit/outbox writes retain atomic rollback. No separate TOTP algorithm is used.

The pinned plugin has no public import/encryption API for historical authenticator
secrets. Retained Rodauth factors use the existing totp-rs compatibility verifier
behind a restricted Better Auth session, capped at five minutes. Until factor
proof, that session cannot enrol credentials, issue recovery codes or enter the
application. Successful proof rotates the framework session and preserves any
remaining onboarding gate. Account locking, replay checks and temporary failure
limits remain mandatory. Framework factor enrolment/disable records a durable
disabled marker for the retained factor; rollback rows are preserved without
allowing them to reactivate the old requirement.

Recovery credential replacement uses the upstream password-reset token generator
and validator without changing the framework token record. A separate record
binds its hash to the recovery session, chosen operation and 30-minute deadline.
An explicit CSRF-protected confirmation consumes both records with the credential
change, audit, notification and session rotation. Email confirmation preserves
any enabled authenticator requirement. Regenerated recovery codes use the same
server API-key issuer as onboarding, with a generation-specific save acknowledgement;
unacknowledged regenerated codes cannot sign in. Replacing that set again needs
another operation-bound fresh proof.

Credential operations share the password/passkey proof dispatcher. Additional
passkey registration consumes a session-bound, short-lived proof grant. Removal
rechecks the current local methods and authenticator state under the account lock.
Removing an adopted password retains an empty credential-provider row, preventing
the preserved Rails rollback hash from being adopted again.

The auth owner is the sole implementation writer. The verifier owns costly builds,
dependency resolution and tests; the coordinator owns review, reports, commits
and publication. Freeze verified source and preserve exact live-job handles.

## Signup, login and recovery

- Offer password or passkey first. Verify email before application access.
  Everyone, including ZITADEL users, retains a local password or passkey.
- Require acknowledgement of saving ten recovery codes before application access.
  Provide copy/download controls and never log plaintext codes.
- Password minimum: 15 characters. Allow long passphrases and password managers;
  no composition or periodic-change rules. Check a versioned local
  common/breached-password blocklist; no remote password lookup.
- Passkeys require user verification and no TOTP. Optional TOTP protects password
  login only. Enabling TOTP requires a password; removing that password requires
  explicitly disabling TOTP first. No remembered-device TOTP bypass.
- Recovery uses separate server-only Better Auth API-key configuration:
  explicit config ID, one remaining use, no refill, no API scopes or session
  emulation. Upstream server APIs generate/hash/verify tokens. A transaction-bound
  store atomically consumes a code, revokes all other browser/native sessions and
  personal keys, records audit and creates the framework session.
- Code login grants normal access, retaining unused recovery codes. Replacing a
  lost credential afterwards requires fresh operation-bound email confirmation;
  subsequent changes use normal fresh authentication.
- Email recovery never bypasses TOTP. Without TOTP or recovery code, an
  MFA-protected account has no self-service recovery.
- Temporary account/source throttling; non-enumerating authentication/recovery
  errors. No indefinite lockout.
- Database purpose enforces enrolment-only sessions: email verification alone,
  credential registration alone and unacknowledged recovery codes grant no clinical
  access. Never use fake passwords for passkey-first signup.

## Sessions and security operations

- Browser/native sessions expire after seven idle days or 30 absolute days.
  Refresh cannot extend the absolute limit. Check authoritative revocation state.
- Every email, credential, TOTP, recovery-code, provider, key creation or closure
  operation requires fresh authentication bound to that single pending operation.
  Reject replay and substitution. Accept fresh passkey, password with applicable
  TOTP or verified fresh ZITADEL proof; existing sessions alone are insufficient.
- Show active sessions; individual revocation and logout are available.
- Verify new local email before activation and notify the old address.
- Closure disables login, revokes credentials/sessions and ends memberships while
  preserving shared care/audit history. Sole owners transfer ownership first.
- Notify credential, MFA, email, provider, API-key changes, recovery and closure.
  Omit routine successful-login notifications.

## ZITADEL and households

- Optional ZITADEL follows local registration/invitation policy and grants no
  household access itself. Auto-link matching accounts only if both emails are
  verified; then identify provider accounts by issuer and immutable subject.
- Synchronise changed verified provider email when unique and notify. Reject
  collisions without account merging.
- Provider handles its own MFA. Sensitive changes require authenticated auth_time
  proving fresh provider authentication, not another existing-session redirect.
- New provider users establish local credentials and save codes before access.
  Provider outages/suspension do not disable local login. MedTracker suspension
  blocks all MedTracker access.
- Verified adults may create multiple households. Preserve clinical permissions
  and account/person distinctions. Owners/admins invite ordinary members; only
  owners grant administrator/ownership powers.
- Seven-day invitations; resend invalidates prior links. Acceptance, membership
  and audit commit atomically. No household MFA policy or admin impersonation.

## Personal API keys

- Choose explicit permissions and households at creation, including optional
  invitation/membership administration. Effective access intersects fixed grants
  with current owner permissions on every request.
- Default expiry 90 days; maximum one year. Grants, households and expiry are
  immutable. Revoke and replace to change them.
- Show secret once; retain name, expiry and last-used details.
- Keys cannot change account security, close accounts, create more keys or become
  browser sessions. Strictly separate recovery-token and personal-key identities.
  Recovery revokes personal keys while retaining unused recovery codes.

## Acceptance, defaults and cutover

Use existing Rust/browser runners with failing behaviour tests before production.
Cover password/passkey/password-TOTP/ZITADEL onboarding/login; enrolment access
gates; concurrent recovery and rollback; credential-type separation; expiry,
revocation, replay and cross-account denial; operation-bound fresh proofs; OIDC
issuer/audience/signature/nonce/state/PKCE/freshness rejection; invitation
expiry/resend/privilege/concurrency; key household isolation and withdrawn owner
permissions; existing native login/refresh/logout/revocation contracts.

Exercise actual queued email delivery and browser journeys on owned fixtures.
Capture desktop/mobile screenshots. Obtain authorised Devin SWE-2 Max security
review and fix verified findings. Run applicable task ci, publish the focused PR
and verify hosted checks. Record CPU/RAM separately for app and tooling. Update
OpenAPI and the existing progress report. Complete means working journeys, review
and hosted checks; compilation alone is insufficient. No merge or deployment.

Verification links: 24 hours. Recovery/email-change proofs: 30 minutes.
Operation-bound fresh-auth proofs: five minutes, single use. Recovery codes remain
valid until consumed, regenerated or closure; regeneration invalidates the set.
Preserve configurable registration and INVITE_ONLY precedence. Rehearse approved
old-session/token and unsupported-credential invalidation on synthetic migrated
accounts, preserving live credentials and rollback data. Production ZITADEL
configuration/activation and live-provider verification remain separate; automated
acceptance uses an owned OIDC fixture.
