# Secure journey entry product report

The secure entry source passed final combined acceptance on 25 September 2026. This report covers the product writer's API and Leptos paths; the full evidence and screenshot inventory are in [journey-auth-runner-report.md](journey-auth-runner-report.md).

## Implemented boundary

- The Rust API now publishes OAuth discovery, public mobile-client capabilities, authorization, login, consent, token exchange, revocation, and account-level household selection. The issued mobile bearer uses the existing medication read authorization path.
- The authorization request accepts an exact registered public mobile client, registered callback, allowed scopes, `code`/`query`, and S256 PKCE. A signed, ten-minute pending cookie carries the validated request and login CSRF value. Login rotates to a signed browser cookie backed by `account_active_session_keys`; active account, user, lockout, factor enrolment, session inactivity and absolute age are checked before consent. Cookies are HttpOnly and SameSite=Lax, and Secure for HTTPS. The configured public origin is the sole source of discovery URLs.
- Password verification runs on a bounded blocking pool. Failed passwords increment the existing account failure counter atomically and lock the account after five attempts. Accounts with OTP, WebAuthn or recovery codes receive an unsupported-method message before a browser session is created. Consent rechecks factor enrolment and CSRF.
- A mobile grant is account-level and contains no fixed household or person. Consent may approve a subset of requested scopes, with `medtracker` required. Code redemption locks the grant, checks registered client, callback and S256 verifier, and consumes the code once. Refresh rotation preserves the original authentication and last user activity times; it replaces the hashed access and refresh tokens under a row lock. Revocation invalidates both. Household selection reads current operational memberships and roles under the restricted database role.
- Leptos renders the API login and consent forms in a responsive split auth shell with a self-hosted stylesheet. Consent explains the two requested scopes in plain language while preserving their OAuth values. Login errors remain visible and move keyboard focus to the password field. Public `/login` without an authorization request shows an information page with no sign-in form. The active login form's `Forgot?` link leads to an explicit unsupported password-reset response.

## Persistence and configuration

Existing SeaORM entities cover ordinary client, account, active-session, factor, household and grant reads, plus browser-session and authorization-grant inserts. Focused SQL remains for atomic failed-login upsert, row-locked code and refresh redemption, and atomic token rotation or revocation. Each request opens a transaction and enters `med_tracker_app`; account tenant settings are transaction-local where needed. OAuth database errors log a generic operation failure without bind values or credential material.

The API requires `AUTH_SESSION_SECRET` of at least 32 bytes and a trusted `PUBLIC_BASE_URL` at startup. The latter must be an HTTPS origin, except explicit loopback HTTP for local tests. Production needs a stable secret shared by replicas. Browser cookies are signed; server-side active-session rows remain authoritative.

## Checks

- `task api:check` passed.
- `task api:fmt` passed.
- `task api:clippy` passed with no warnings from this code.
- `task api:test` passed: three API unit tests.
- `task -d rust/web test` passed: one web unit test.
- `task -d rust/web lint` passed.
- `task -d rust/web fmt` passed.
- `git diff --check` passed at product freeze.

The existing six OAuth HTTP cases established a genuine red against the unchanged API before implementation. Final combined acceptance passed all 30 selected HTTP cases, including OAuth and prior medication regressions, and all five browser cases with no skips. The runner verified an unchanged 1,499-path runtime input manifest before and after the final run (SHA-256 `36e580245342cce33ec66813f652cb504536185fa48398399334bcce22d13b59`). Desktop/mobile screenshots show readable login and consent forms with plain-language scope descriptions. The UI changes did not alter auth logic; see the [runner report](journey-auth-runner-report.md) for exact command, fixture, image and screenshot evidence.

## Bounded gaps

OTP, passkey and recovery-code completion are not implemented in Rust. Password-only sign-in for enrolled-factor accounts fails closed. Password reset and consent decline are also not implemented. The Rust browser session format is separate from Rails Rodauth sessions, so users must sign in again when crossing implementations. This slice does not complete dose recording, stock/history, or full authentication parity.
