# Loco identity implementation decision

Updated 6 October 2026. This records the implemented architecture and the account
verification adapter being added. It does not establish whole-identity or release
acceptance; the remaining signup, native-client and cutover gates still apply.

## Maintained implementations

Use the root application's existing dependencies and preserved account schema:

| Concern | Implementation |
| --- | --- |
| OAuth grant state transitions | oxide-auth 0.6.1 and oxide-auth-async 0.2.1 |
| Retained password and confidential-client hashes | bcrypt 0.19.3 |
| Persistent browser sessions | axum_session 0.21.0 and axum_session_sqlx 0.11.0 |
| Browser CSRF | axum_csrf 0.11.0 and the existing origin checks |
| Authenticator codes | totp-rs 6.0.0 |
| Passkey validation | webauthn-rs-core 0.5.5 |
| Stored email-token compatibility | Maintained getrandom, HMAC-SHA256, URL-safe Base64 and subtle comparison |

The application supplies transactional SeaORM storage, registered clients,
current account and household authority, and audit effects. It does not introduce
an external issuer, a second grant engine or new cryptographic primitives.
`tests/identity_compatibility.rs`, `tests/oauth_server.rs` and
`tests/identity_resource.rs` contain the stored-format, HTTP, concurrent-redemption,
rotation, revocation and scope cases. Browser tests exercise actual sign-in,
consent, MFA and passkey continuation. Their presence is not a substitute for the
required final runtime and hosted results.

The owner-approved unsupported-passkey transition is recorded in
[the identity slice](slices/03-identity.md) and
[the deployment guide](../../deployment.md). Supported retained keys must continue
to work; affected users use verified recovery and replacement without bypassing MFA.

## Account verification compatibility

Loco 1.2.0's runtime `src/auth/mod.rs` exports JWT authentication. Its hashing
helpers use Argon2id. The generated authentication starter defines
`users.email_verification_token`, `email_verification_sent_at` and
`email_verified_at`; these are different from MedTracker's preserved accounts,
users and `account_verification_keys`. The Loco authentication tutorial describes
verification as generated application code and requires the application to enforce
verified status. There is no reusable runtime Rodauth-verification capability to
configure against the preserved schema.

Use Loco's standard mail worker and the existing session, CSRF and transaction
boundaries. Add only the stored-format adapter required for retained Rodauth
verification links. Rodauth 2.48.0's `verify_account.rb` creates a random key by
account ID and allows verification only while the account is unverified. Its
`email_base.rb` serialises the account ID, an underscore and the transformed key;
validation parses that ID and compares the configured current or old HMAC in
constant time. `base.rb` uses 32 random bytes, URL-safe Base64 and HMAC-SHA256.
Maintained crates perform all random generation, hashing, encoding and comparison.

Preserve the status transition from unverified `1` to verified `2` and the
one-use key semantics. Retained verification has no key deadline column or default
deadline; do not invent one. Invitation expiry remains independently enforced.
An unverified account cannot obtain a clinical browser principal.

Signup creates its canonical Person and User directly in the invited household.
Its membership, invitation grants and acceptance effects share the same transaction
and reuse the invitation domain implementation. Existing-account acceptance keeps
its separate household-local person copy. Do not fabricate a browser principal or
duplicate the canonical signup person to invoke these effects.

Required runtime proof includes actual verification mail delivery, retained token
interoperability, invalid and used keys, current account status, invitation
binding, duplicate and cancelled invitations, password clearing, CSRF and origin
rejection, atomic effects and immediate usability after verification. Source and
mail-queue insertion alone do not prove acceptance.

Primary references: the pinned Loco and Rodauth source above and the current Loco
[authentication tutorial](https://loco.rs/docs/tutorials/saas-with-auth/) and
[password hashing guide](https://loco.rs/docs/how-to/hash-passwords/).
