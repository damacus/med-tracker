# Proposal

## Why

Existing MedTracker accounts with passkeys or authenticator apps cannot finish sign-in through Rust, and Rust password reset is unavailable. Complete the Rails authentication behaviour in bounded, independently verified slices so account credentials and medical history survive a future cutover.

Related originating migration work: [#2299](https://github.com/damacus/med-tracker/issues/2299). Native authentication context: [#1889](https://github.com/damacus/med-tracker/issues/1889). This proposal does not claim either issue is finished.

## What Changes

- Prove compatibility of existing Rodauth OTP secrets, replay rules and passkey records before enabling existing-factor login.
- Complete password, passkey, optional OTP and recovery-code sign-in, including credential management and trusted authentication assurance.
- Complete configured generic OIDC sign-in while retaining account identities, invitation rules and existing local login methods.
- Complete registration, verification, password reset/change, login change, unlock and account closure without deleting medical history.
- Preserve signed database-backed browser sessions and account-level native S256 PKCE, consent, refresh rotation and revocation.
- Preserve the accepted 30-day interactive inactivity default, optional absolute maximum, and removal of action-specific fresh-MFA gates. Credential-management and login MFA protections remain required.

Non-goals: replacing identity platforms, changing account IDs, deployment, enabling a factor from an unverified import, reintroducing action-specific MFA freshness, changing household permissions, or claiming performance improvement without measurement. The first implementation slice proves synthetic OTP compatibility only; broad parity remains open.

## Capabilities

### New Capabilities

- `rust-authentication-compatibility`: Existing credential compatibility and assurance-preserving browser/native sign-in.
- `rust-account-lifecycle`: Existing account registration, recovery and closure behaviour in Rust.

### Modified Capabilities

None. The active `unify-mobile-login-with-rodauth` change remains the native authentication policy reference; its accepted requirements are preserved.

## Impact

Future implementation affects `rust/api`, Rust browser views and contract fixtures/tests, while Rails code and `db/schema.rb` remain compatibility references. Proposed protocol primitives are `webauthn-rs` 0.5.5, `totp-rs` 6.0.0 and `openidconnect` 4.0.1; all require import and runtime proof before deployment. Legacy OTP derivation requires the existing Rails HMAC secret through an explicitly configured secret boundary. This change introduces no live-secret access or database migration in the initial slice.
