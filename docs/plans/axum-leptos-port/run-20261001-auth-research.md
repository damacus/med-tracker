# Authentication research — 1 October 2026

## Recommendation

Extend the existing Rust authentication boundary with focused primitives and explicit MedTracker workflows. Keep account IDs, existing authentication tables/bcrypt hashes, signed database-backed sessions and native OAuth endpoints. No researched crate provides complete Rodauth parity while retaining this model.

| Concern | Recommended primitive | Compatibility and remaining work |
| --- | --- | --- |
| OTP | `totp-rs = 6.0.0`, MIT, Rust 1.88 | Framework independent; v6 Builder/Totp API returns matched step. MedTracker owns replay state, failures, enrolment/removal and recovery codes. |
| Passkeys | `webauthn-rs = 0.5.5`, MPL-2.0, Rust 1.88 | Server-side assertions/registration; existing 64-byte handles, encoded CBOR/COSE and missing credential metadata require a separate proof. |
| Generic upstream OIDC | `openidconnect = 4.0.1`, MIT, declared Rust 1.65 | Discovery, code exchange, signature/claims/nonce/PKCE primitives; identity linking, invitations, assurance and logout stay application-owned. |
| Account lifecycle | Existing bcrypt, SeaORM, getrandom, HMAC/SHA256 and lettre | Verification/reset/change/unlock/closure workflows and transactional concurrency remain application-owned. |

Official published manifests: [totp-rs](https://docs.rs/crate/totp-rs/6.0.0/source/Cargo.toml), [webauthn-rs](https://docs.rs/crate/webauthn-rs/0.5.5/source/Cargo.toml), [openidconnect](https://docs.rs/crate/openidconnect/4.0.1/source/Cargo.toml). Current upstream manifests agree with those versions. [TOTP releases](https://github.com/constantoine/totp-rs/releases) include v6 API/MSRV changes; [OIDC releases](https://github.com/ramosbugs/openidconnect-rs/releases) include the 4.0.1 SHA512 hashing fix. WebAuthn published-source and current-workspace evidence establishes the reviewed version; a current 0.5.5 release date was not verified from GitHub's release listing. Maintenance evidence is an observed snapshot, not a guarantee of future support.

Repository evidence: `rust/contract-tests/Dockerfile` uses Rust 1.98.1; `rust/api/Cargo.lock` resolves Axum 0.8.9, and `rust/api/Cargo.toml` uses Reqwest 0.13. OIDC's OAuth2 5 integration targets Reqwest 0.12. Prefer its documented [AsyncHttpClient](https://docs.rs/oauth2/5.0.0/oauth2/trait.AsyncHttpClient.html) transport adapter or a separately scoped 0.12 client, with redirects disabled. Keep validation inside the OIDC crate. [OAuth2 manifest](https://raw.githubusercontent.com/ramosbugs/oauth2-rs/5.0.0/Cargo.toml), [OIDC API](https://docs.rs/openidconnect/4.0.1/openidconnect/).

## Exact credential compatibility

Locked dependencies in Gemfile.lock are Rodauth 2.48.0, Rodauth Rails 2.2.2, WebAuthn 3.4.3 and ROTP 6.3.0. Source was inspected in the matching locally installed gems; no live account data or secret was read.

OTP storage is ordinarily a seed, not the enrolled authenticator secret. Rodauth Rails defaults HMAC secret to Rails secret_key_base. Rodauth decodes the stored 16/32-character lowercase Base32 seed, computes HMAC-SHA256, then maps the first seed-length digest bytes modulo 32 into `abcdefghijklmnopqrstuvwxyz234567`. This custom encoding is **not** standard Base32 encoding of HMAC bytes. Preserve exact current/old-secret rotation behaviour. A previous preliminary shorthand describing this as Base32(HMAC) was insufficient; this report supersedes it.

Rodauth sets drift to 30 seconds and uses ROTP verification with a strictly-after last_use constraint. Its atomic interval gate also requires `last_use + 30 seconds < database now`. The OTP row stores last_use and a five-failure counter. Matched-step verification alone does not supply the row lock and persisted last-use mutation.

Sixteen-character legacy seeds decode to 80 bits. The regular totp-rs 6.0.0 builder requires at least 128 bits; compatibility verification needs its explicit `build_noncompliant` path after strict legacy-seed validation and fixed valid algorithm/digits/step settings. This exception applies to importing existing factors and does not authorise generating new short secrets. Full-precision database timestamps, atomic updates and future-drift reuse behaviour remain integration gates.

WebAuthn.generate_user_id generates 64 random bytes, encoded into account_webauthn_user_ids.webauthn_id. High-level webauthn-rs registration/discoverable identification expects UUIDs. Preserve the existing handle bytes, credential IDs, public keys and sign counters. Rails public keys are encoded CBOR/COSE; serialized Rust Passkey values include additional metadata absent from the Rails schema. Do not invent attestation, backup or UV history. [Passkey import API](https://docs.rs/webauthn-rs/0.5.5/webauthn_rs/prelude/struct.Passkey.html), [discoverable API](https://docs.rs/webauthn-rs/0.5.5/webauthn_rs/struct.Webauthn.html).

Prefer the safe WebAuthn wrapper if a separate spike proves compatibility. Its [core API](https://docs.rs/webauthn-rs-core/0.5.5/webauthn_rs_core/struct.WebauthnCore.html) can accept byte handles but explicitly warns that callers own additional invariants and signatures may change within minor versions. Direct core use remains a reviewed conditional option, not an approved login implementation.

Recovery codes are existing readable strings, consumed with constant-time comparison and a delete affecting exactly one row. Preserve unused codes and current viewing semantics. Hashing codes changes that behaviour and requires a separate decision. Closing an account clears person.account_id but preserves person and medical history; nullable password hashes and partial unique email indexes for account statuses 1/2 must survive.

## Current gaps and policy

Rust oauth.rs returns 501 for password reset. Password login and browser-session lookup reject accounts with OTP/passkey/recovery rows, intentionally failing closed. BrowserSession has no factor-assurance fields. Rust counts recovery-only rows as requiring MFA, while Rails removes recovery as an offered method when no primary factor exists.

Rails rodauth_main.rb enables local password, OTP/recovery, passkey/autofill, generic OIDC and lifecycle methods. It requires verified resident passkeys and credential-modification password checks. Existing-factor enrolment and recovery viewing retain MFA checks. `unify-mobile-login-with-rodauth` removes action-specific fresh-MFA gates and keeps 30-day inactivity/no default absolute maximum. Rust already advertises fresh_mfa_required:false. Do not reintroduce obsolete action freshness when completing login assurance.

Useful source/test boundaries: `rust/api/src/oauth.rs`, `rust/api/src/auth_sessions.rs`, `app/misc/rodauth_main.rb`, `app/services/authentication_lifetime.rb`, `db/schema.rb`, `spec/requests/webauthn_setup_spec.rb`, `spec/security/oidc_security_spec.rb`, `rust/contract-tests/tests/oauth.rs`, and `openspec/changes/unify-mobile-login-with-rodauth/design.md`.

## Alternatives considered

`axum-login` 0.18.0 is MIT and fits Axum 0.8, but introduces tower-sessions while still requiring application authentication backends. It increases session migration work without implementing the missing factor/lifecycle behaviour. [Manifest](https://docs.rs/crate/axum-login/0.18.0/source/Cargo.toml), [API](https://docs.rs/axum-login/0.18.0/axum_login/).

1Password passkey 0.5.0 implements clients, authenticators and transports, not the relying-party server needed here. It is not a substitute for server WebAuthn verification. [API](https://docs.rs/passkey/0.5.0/passkey/).

## Proof and acceptance gates

1. Synthetic Rails-created OTP/passkey/recovery credentials work with unchanged account IDs; malformed, cross-account, replayed and expired submissions fail.
2. OTP preserves accepted drift and last-use interval/failure behaviour; concurrent OTP/recovery use permits at most one success.
3. OIDC rejects bad signatures/issuer/audience/nonce/state, retains linked IDs and invitation restrictions, and accepts provider MFA only from validated acceptable claims.
4. Signup/verification/reset/change/unlock/closure preserve rollback, one-use expiry, safe responses, after-commit mail and retained medical history.
5. Session revocation/expiry, CSRF/origin checks, native S256/consent/code/refresh/revoke and current household/person authorisation remain green.
6. Real provider/device/browser acceptance and independent review gate cutover. No speed or memory gain has been measured.
