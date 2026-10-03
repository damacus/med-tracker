# Rust security investigation

Baseline: `7a8e8858` (3 October 2026). Rails remains the behaviour reference.

## Remediation design

Replace WebAuthn assertion and client-data interpretation with a maintained
WebAuthn implementation. Preserve required user presence and verification,
exact origin and RP binding, and strict nonzero counter monotonicity. Keep
Rails credential IDs, COSE storage and user handles at a narrow adapter boundary.
Do not manufacture registration attestation or claim that migrated keys were
attested. Library algorithm support must be checked against existing Ruby
credentials. There are no existing Rust users, so Rust cookie migration is
unnecessary. Library-supported algorithm expansion is welcome when it verifies
correctly; do not preserve a historical Rust allowlist for its own sake.

Keep challenge consumption, credential verification/update and session insertion
in one database transaction. Lock the credential before reading its counter.
Exercise both same-challenge replay and distinct-challenge concurrent assertions;
the existing unique challenge digest only addresses the former.

Use one typed pagination boundary for complete collections and dashboard history.
Reject inconsistent totals, page numbers, page sizes and duplicate IDs. Preserve
JSON parsing errors internally while returning a generic gateway failure.

Retain sound household locking and reauthentication. Avoid restructuring unrelated
domain modules. Replace wildcard dependency imports in authentication modules as
they are changed.

## Initial evidence

- The baseline Rust API suite passes, including 13 WebAuthn tests, all using ES256.
- Three new negative tests fail: cross-origin assertions, mismatched credential
  IDs/types, and backup-state flags without backup eligibility are accepted.
- Challenge replay already has an atomic unique-digest insertion in commit
  `a349bdc9`. It is bound to signed CSRF context and a ten-minute lifetime.
  Successful login commits consumption and session together; failures roll back.
- The credential lookup has no row lock. Distinct challenges can bypass the
  challenge uniqueness fence while observing the same old counter.
- The dashboard history walker does not validate page metadata or duplicate IDs;
  the general collection walker does. History is ordered by ascending row ID, so a
  taken-at cutoff cannot safely terminate that walk early.

## Library investigation

- `webauthn-rs` / `webauthn-rs-core` 0.5.5: established server implementation,
  typed assertions, COSE parsing, origin/flag/counter checks and OpenSSL verification.
  Version 0.5.5 fixes GHSA-22w3-693w-x895. Subdomain and arbitrary-port matching
  must remain disabled. Source inspection shows verification supports ES256,
  RS256 and EdDSA, not all algorithms enumerated by its protocol types. COSE RSA
  import also constrains modulus/exponent sizes. Rails advertises PS256 by default.
  The user chose library-only support and accepted that unsupported Ruby passkeys
  will need re-registration. No fallback verifier is retained.
- `oauth2` and `openidconnect` are client libraries, not replacements for the
  application's OAuth authorization server. No Rust OIDC relying-party or ID-token
  verification flow was found. JWT signing for push integrations already uses
  `jsonwebtoken`; password verification uses `bcrypt`; TOTP uses `totp-rs`.
- `oxide-auth` is an OAuth server candidate. Adoption requires adapters for the
  shared Rails grant tables, transactional one-use codes, refresh rotation,
  account availability and existing response behaviour. Review PKCE support and
  persistence interfaces before changing these paths.

Sources: [WebAuthn advisory](https://github.com/kanidm/webauthn-rs/security/advisories/GHSA-22w3-693w-x895),
[WebAuthn core source](https://docs.rs/crate/webauthn-rs-core/0.5.5/source/src/crypto.rs),
[Rails WebAuthn](https://github.com/cedarcode/webauthn-ruby),
[oxide-auth](https://docs.rs/oxide-auth/latest/oxide_auth/),
[openidconnect](https://docs.rs/openidconnect/latest/openidconnect/).

## Confirmed findings

| Finding | Before-fix evidence | Change |
| --- | --- | --- |
| Credential counter race | Two distinct challenges with the same positive counter both returned 302 under a controlled database interleaving | Credential `FOR UPDATE` covers verification, counter comparison/update, consumption and session creation |
| Counter truncation | A counter above `i32::MAX` signed in while persistence clamped it | Reject unrepresentable counters and negative stored counters |
| Discoverable user binding | Username-less login accepted a missing `userHandle` | Require the handle and bind it to the key's account |
| Assertion validation gaps | Tests accepted cross-origin assertions, inconsistent ID/type and invalid backup flags | Typed library parsing plus explicit missing invariants |
| PKCE syntax | Authorization accepted overlong/noncanonical S256 challenges | Canonical 32-byte challenge decoding and library verification |
| Claim purpose | Identically shaped claims could be verified under another purpose | Purpose-bound `cookie::PrivateJar` replaces custom HMAC framing |
| Membership TOCTOU | Create returned 201 and update returned 200 after membership revocation while waiting on the household lock | Reauthenticate after locking, before policy checks/writes |
| Pagination completeness | Dashboard history lacked consistency checks | Shared typed metadata and duplicate/count/page checks |
| JSON diagnostic loss | Malformed internal API JSON became null | Preserve parse errors internally and return a generic 502 |

Purpose confusion was demonstrated at the envelope boundary, not as an account
takeover: the original claim structures already differed. Origin normalisation
was a regression caught during library adoption and fixed: paths, credentials and
queries cannot become valid assertion origins. Ed448 is accepted through the
library, with valid-signature and tampering coverage.

## Disproved and qualified concerns

Challenge replay protection already existed. The regression reuses the original
login cookie and the successful zero-counter assertion, so rejection cannot be
attributed to clearing the cookie. Unique consumption and session creation share
a transaction. The digest now identifies the authenticated logical challenge
rather than its envelope encoding.

Randomness comes from OS-backed UUID v4 generation, with three UUIDs hashed into
the challenge. Challenges expire after ten minutes and bind to an authenticated
login/OAuth CSRF context. Discoverable login cannot bind an account before key
selection; the key and required handle establish that account afterwards.

Existing stock operations already lock household, medication and dosage in that
order, reauthenticate after locking and keep stock/version/audit/sync/idempotency
effects in one transaction. Those mechanisms are retained. OAuth code redemption
and refresh rotation already lock grant rows; redemption clears the code.

No Rust OIDC relying-party flow, ID-token validator or JWK-set parser was found.
Bearer tokens are opaque database-backed credentials, not JWTs. JWT signing,
password hashing, TOTP and Web Push already use established crates.

## Inventory and library decisions

Recent commits implement Rails migration parity. No recorded library-selection
justification was found for the original manual WebAuthn or cookie code. Shared
storage explains format adapters, not a need to recreate the protocol.

| Area and code | Decision | Remaining responsibility and migration cost |
| --- | --- | --- |
| Assertion/client/authenticator data, `webauthn.rs` | Adopt `webauthn-rs-core = 0.5.5` typed data and authentication verification | Supplement cross-origin/ID/type/backup invariants not all enforced by the authentication entry point |
| Rails credential import/counters | Library COSE parsing and `CredentialV3` conversion | A typed serialisation adapter builds library state from an authenticated challenge and locked COSE credential. Its pinned representation needs upgrade tests; attestation is not fabricated |
| Algorithms/key shapes | Delegate selection, COSE key parsing and signatures entirely to WebAuthn core; remove the direct ring dependency | Core cannot verify PSS/P-384 and constrains RSA import sizes. The approved migration rejects those credentials through the library; re-registration may be required. No custom allowlist or compatibility verifier remains |
| Challenges, `oauth/login.rs`, `oauth/passkey.rs` | OS randomness, authenticated cookies, PostgreSQL uniqueness/transactions | Context binding, expiry and atomic session/counter/consumption are application persistence policy |
| OAuth server, `oauth/authorization.rs`, `oauth/tokens.rs` | Investigated `oxide-auth`; adopt its PKCE extension | Full orchestration needs Registrar/Authorizer/Issuer adapters preserving shared Rails grants, row-locked one-use codes, rotation and account/response behaviour. This remains protocol maintenance debt |
| OAuth/OIDC clients | Investigated `oauth2` and `openidconnect` | These are clients, not authorization-server replacements. No client or ID-token flow exists to replace |
| PKCE, `oauth/pkce.rs` | `oxide-auth 0.6.1`, `Pkce::required` | Enforce S256, verifier syntax and canonical challenge shape before the library; it does not impose all these constraints itself |
| Cookies, `oauth/configuration.rs` | `cookie 0.18` private jar/key expansion | Purpose is authenticated as the cookie name. Flags, expiry and account/session checks remain application policy. Old Rust cookies intentionally become invalid |
| Sessions, `oauth/sessions.rs`, `auth_sessions.rs`, `request_authentication.rs` | Typed SeaORM models, library digests and authenticated cookies | Shared Rails session/grant/membership rows carry lifetime, revocation and permission versions. Generic session storage is not a drop-in substitute. Invalid explicit bearers must never fall back to cookies |
| CSRF/state/nonce | Purpose-bound claims, random synchroniser token, origin checks, exact redirect allowlist | Route-aware cookie/bearer policy remains. OAuth clients validate returned state. There is no OIDC nonce; generic double-submit middleware does not replace account/context binding |
| Password/OTP/recovery, `oauth/login.rs`, `auth_compatibility.rs` | Retain `bcrypt`, `totp-rs`, `hmac` and locks | Rodauth secret derivation, old-secret rotation, replay windows and recovery persistence are compatibility glue. Changing derivation breaks Ruby OTP credentials |
| JWT/JWK, `external_integrations/push.rs` | Retain `jsonwebtoken` for ES256 APNs and `web-push` for VAPID | No custom JWT decoder/JWK parser to remove |
| Portable encryption, `portable_crypto.rs` | Retain `aes-gcm`, `pbkdf2`, `getrandom` | Rails MessageEncryptor-compatible envelope and KDF parameters are versioned format glue; changes need export migration and interoperability tests |
| Occurrence tokens, `dose_occurrences/keys.rs` | Retain `hmac` constant-time verification | Domain MAC binds source/date/position with a distinct version prefix. It is not a login token; JWT/OAuth would change the wire format without replacing domain checks |

Rails stores no historical backup-eligibility metadata. The adapter checks current
flag consistency but cannot detect historical eligibility transitions. That needs
a schema migration. The signed counter column cannot store all `u32` values;
failure is now closed rather than silently weakening replay detection.

The baseline accepted ES256 `-7`, ES384 `-35`, Ed25519/EdDSA `-8`,
RS256/384/512 `-257/-258/-259` and PS256/384/512 `-37/-38/-39`.
The matrix generates valid and tampered assertions for every baseline algorithm.
Core 0.5.5 accepts ES256, RS256 with its supported key shape, Ed25519 and Ed448;
the other algorithms fail through the library. Ed448 has positive/tampering
coverage. Production code contains no independent algorithm allowlist: support
may expand with library upgrades. Tests document the pinned version's behaviour.
Rails advertises ES256, PS256 and RS256 by default, so the approved library-only
choice can require re-registration of existing Ruby PS256 passkeys. Existing
RSA keys outside core's 2048-bit modulus/three-byte exponent importer can also
require re-registration. This is an intentional compatibility change, not a claim
that all Ruby passkeys migrate unchanged.
Before Rust cutover, enrollment must select library-supported algorithms;
re-registering through an unchanged Rails configuration is not guaranteed to
choose a different algorithm. That enrollment decision belongs to the follow-up.

## JSON, completeness and boundaries

Forms were already typed. Assertion/client data and pagination metadata now cross
typed deserialisation boundaries. Resource patches remain dynamic where Rails
distinguishes missing fields, null, decimal strings and per-field validation
errors. Plain `Option<T>` would collapse missing/null semantics. Field allowlists,
role checks and transaction boundaries were retained.

Other null fallbacks were classified: optional fields and version-diff absences
are intentional; invalid invitation bodies fail validation; push-provider parse
failures become transient delivery failures. Stock-removal historical JSON still
loses parse diagnostics. Corrupt replay data fails equality checks, but history
can show null fields. A follow-up needs corruption fixtures and a defined history
error response rather than silently changing retained Rails history.

Both walkers reject duplicate IDs and inconsistent metadata. They cannot guarantee
snapshot consistency against concurrent same-count edits; that needs a server
snapshot/cursor contract. Errors are visible rather than returning plausible
incomplete collections.

History sorts by ascending ID, not `taken_at`; imports/backdating put recent takes
anywhere. The dashboard needs current week/month takes and the latest older take
per schedule/assignment for interval calculations. It still reads every page.
Payload retention is limited to useful takes, but duplicate IDs use O(total
history) memory. No production volume was available, so no claim is made that this
is cheap at arbitrary scale. Measure and add a server aggregate/snapshot before
removing the scan; a timestamp early exit would be wrong.

Changed authorization, token, passkey and API-client modules declare dependencies
explicitly. Untouched OAuth and domain modules retain some parent dependency bags
and wildcard imports. A wholesale import rewrite would not enforce another
invariant in this change.

## Dependencies and follow-ups

Added WebAuthn core 0.5.5, `serde_cbor_2`, `oxide-auth` without defaults and private
`cookie` features. API algorithm tests use OpenSSL; database assertion fixtures
use P-256/SHA-256. Core introduces an OpenSSL build/runtime dependency, exercised
by the Linux contract build. Cookie brings older stable RustCrypto versions
alongside newer existing versions; those duplicate versions are deliberate.

Baseline and changed API locks report the same `RUSTSEC-2023-0071` RSA advisory
and unmaintained `paste`, `proc-macro-error`, `proc-macro-error2` warnings. No new
advisory was introduced; this is not a clean audit. RSA comes from `jsonwebtoken`
and `web-push` → `jwt-simple` → `superboring`. APNs/VAPID use EC; WebAuthn public
RSA verification uses the WebAuthn library's OpenSSL backend. No affected RSA private-key operation was
found on these application paths. Track the advisory (no fixed release reported),
without suppressing it or claiming a dependency fix.

The full gate refreshes the web lock for its already-declared `tower-http 0.7`
dependency; UI dependencies retain their separate `0.6` version.

Failing-before-fix tests demonstrated assertion gaps, PKCE syntax, purpose
separation, credential races, overflow, missing handle and membership races.
Zero-counter replay already passed. Final gate and database/browser results are
recorded in the delivery report; production was not scanned.

Follow-ups: full OAuth framework/shared-table adapters; counter/registration
metadata migration; corrupt-history diagnostics; dashboard volume/snapshot work;
existing dependency advisories. These are tracked in
[issue #2382](https://github.com/damacus/med-tracker/issues/2382).
Ruby users reportedly sign out within an hour.
Source uses configurable day-based inactivity/absolute limits, which does not
explain the live observation alone. Diagnose against actual configuration.

`AGENTS.md`/`agents.md` now require established security implementations,
documented concrete exceptions and interoperability/negative tests. The broader
rule requires checking proven dependencies before non-trivial infrastructure work.

Further sources: [private cookies](https://docs.rs/cookie/latest/cookie/struct.PrivateJar.html),
[RSA timing advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html).

## Reproducible verification

- `task ci:rust-port`: full UI/API/web format, lint, tests/build and contract-target
  compilation; passed on the final library-only implementation.
- `task ci:test`: 79 CI policy/script tests passed, including the updated login
  job's passkey security target.
- `task api:contract-household-selector-test`: passed.
- `task docs:build` and `git diff --check`: passed.
- `cargo audit` through a temporary Taskfile: nonzero on baseline and final locks
  for the identical advisory/maintenance findings documented above.
- Database/browser regression command:

```sh
task api:browser-rust HOUSEHOLD_ACCEPTANCE=true \
  'HOUSEHOLD_TEST_FILE=passkey_security --test household_stock_concurrency --test oauth --test web_session_api --test medication_management_security_api' \
  BROWSER_TEST_FILES=tests/login.smoke.test.mjs
```

The session-renewal regression checks stable database session IDs and CSRF, not
identical encrypted cookie bytes: fresh authenticated-encryption nonces are
expected. It retains copied-cookie revocation checks after logout.

The final database run passed all 35 tests across credential security, household
stock concurrency, medication authorization, OAuth and browser-session APIs.
