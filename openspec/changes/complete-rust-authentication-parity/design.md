# Design

## Context

See proposal.md for motivation. `rust/api/src/oauth.rs` already implements signed database-backed browser sessions and account-level native S256 PKCE, consent, one-use codes, refresh rotation and revocation. Enrolled-factor accounts fail closed; reset-password request returns 501. Rails `app/misc/rodauth_main.rb`, `db/schema.rb` and the active `unify-mobile-login-with-rodauth` change are the compatibility and policy reference.

## Goals / Non-Goals

Deliver independently verified authentication slices without moving account identities, replacing database ownership or weakening household authorisation. First prove pure legacy OTP compatibility with synthetic data. Existing-factor login, account mutations, provider callbacks and deployment are outside that first slice.

## Decisions

### Reuse the existing application boundary

Keep Axum 0.8, SeaORM and the current signed session format/database revocation boundary. Factor completion must later record trusted assurance before browser sessions and native consent can proceed. Session invalidation must distinguish an enrolled factor from an unsatisfied factor. Recovery-only rows do not become a primary factor.

Rejected alternative: `axum-login` 0.18.0 fits Axum 0.8 but adds tower-sessions while leaving factor/account workflows application-owned. It adds migration work without closing the identified gaps. No identity-platform replacement is assumed.

### Use dedicated protocol primitives

Recommend `totp-rs` 6.0.0 (MIT, declared Rust 1.88), `webauthn-rs` 0.5.5 (MPL-2.0, Rust 1.88), and `openidconnect` 4.0.1 (MIT, Rust 1.65). Repository contract builds use Rust 1.98.1. Pin reviewed versions and verify dependency/licence policy and builds before integration. The research report records official source links and known API gaps.

OIDC delegates discovery and cryptographic/claim validation to openidconnect. A bounded HTTP adapter is needed because its OAuth2 5 dependency uses Reqwest 0.12 while MedTracker uses 0.13; a separate 0.12 client is the alternative. The adapter transports messages and does not implement OIDC validation. Configured issuer/callback rules, identity linking, invitation checks and provider MFA claims remain application decisions. Do not turn MedTracker into an OpenID Provider to expose native OAuth discovery.

### Preserve exact legacy OTP derivation

Locked Rodauth 2.48.0 reads a lowercase Base32 seed of 16 or 32 characters from `account_otp_keys.key`. With Rodauth Rails' default HMAC secret, it decodes that seed, computes HMAC-SHA256 using Rails `secret_key_base`, then maps each of the first seed-length digest bytes modulo 32 into `abcdefghijklmnopqrstuvwxyz234567`. This is not standard Base32 encoding of the HMAC. The resulting string is the authenticator secret. Raw stored-seed decoding is compatible only when HMAC was explicitly disabled.

The initial pure adapter takes synthetic/configured secret bytes explicitly; it reads no environment variables, database or live secrets. It handles current and old HMAC secrets as separate caller-supplied inputs. It must reproduce Rails/ROTP vectors before any route uses it. `totp-rs` v6 returns a matched step, permitting an explicit acceptance decision. Preserve Rodauth's +/-30-second drift, code timestamp strictly after last_use, and database interval gate: `last_use + 30 seconds < now`. Database authentication later locks the row, updates last_use/failure state and establishes assurance in one transaction. A helper returning a matched step cannot replace that transaction.

Sixteen-character legacy seeds decode to 80 bits, below the regular totp-rs builder's 128-bit minimum. The import helper uses `build_noncompliant` only after validating existing 16/32-character seed syntax and fixing valid algorithm, digits and step settings; new short-factor generation is not authorised. ROTP chooses the latest matching step if a code collides inside its window, so verification scans explicit zero-skew steps and retains the latest match. Rodauth persists actual database time rather than that matched counter. Full-precision interval checks, future-drift reuse and concurrent consumption require separate integration tests.

Rejected alternative: treating the stored seed as the enrolled secret or using standard Base32 HMAC encoding. Both silently invalidate existing authenticator apps. New OTP policy or changed secrets require a separate approved change.

### Gate passkey import independently

Rails WebAuthn 3.4.3 creates 64-byte user handles, encoded into `account_webauthn_user_ids.webauthn_id`. High-level webauthn-rs APIs use UUID handles. Rails stores encoded CBOR/COSE public keys, sign_count and credential IDs, while webauthn-rs has a richer serialized credential format. Preserve every existing handle and credential identifier byte-for-byte. Do not invent backup, UV or attestation metadata missing from the database.

A separate synthetic Rails-to-Rust spike must establish registration, discoverable/autofill login, required UV, origin/RP matching, replay and counter handling. Prefer the safe wrapper when proved compatible. Direct `webauthn-rs-core` is a conditional alternative requiring dedicated review because its documentation warns that callers must uphold additional invariants and APIs may change within minor versions. Enabling existing-factor login remains gated on this evidence and session assurance.

### Keep recovery and lifecycle mutations application-owned

Existing recovery strings are compared in constant time and consumed with a delete affecting exactly one row. Preserve existing viewing semantics; a later hashing change cannot be smuggled into compatibility work. Credential enrolment/removal and viewing retain password and enrolled-MFA checks. Action-specific fresh-MFA gates remain removed.

Registration and recovery workflows use the existing tables and atomic transactions. Token consumption/expiry is checked under the appropriate lock. Mail is queued after commit. Account closure retains people/medical history while clearing the account link. Preserve audit event meanings and PHI-safe failures. Login grants account identity only; every household/person request continues current permission checks.

## Risks / Trade-offs

- Legacy secret not configured or old-secret rotation omitted -> fail closed and prove synthetic vectors before deployment; never silently replace enrolments.
- Pure OTP helper mistaken for complete MFA -> leave routes untouched until DB concurrency, failure limits and assurance are covered.
- UUID-based passkey import loses existing user handles -> separate import gate; no re-enrolment assumption.
- Recovery/password requests race -> transactional one-use consumption with concurrent acceptance tests.
- Account linking trusts an unverified email -> validate provider claims and test linking separately from cryptographic exchange.
- Broad parity claimed from a successful spike -> checked tasks and reports distinguish pure compatibility proof from enabled workflows.

## Migration Plan

1. Record synthetic compatibility evidence and dependency decisions.
2. Implement factor, provider and lifecycle slices behind explicit acceptance gates.
3. Validate existing credential formats and identifiers without changing them; keep Rails as the observable behaviour oracle.
4. Complete native/browser acceptance and independent review before considering cutover. Deployment requires separate authority.
5. Keep rollback possible to Rails without modifying medical history or deleting credentials. Sessions/tokens may require reauthentication at a future coordinated cutover; that does not authorise account or credential loss.
