# Household credential prerequisite

Before claiming full dosage or subsequent household-operation parity, resolve
the shared credential gap. `rust/api/src/lib.rs#authenticate` currently looks up
an API session, then falls back to mobile OAuth. It has no household app-token
branch, although `Api::V1::BaseController` accepts `ApiAppToken`. Account-level
session management now accepts app tokens, but that is a separate code path.

Preserve the existing active-account/user, lockout, membership, operational
household, permissions-version, expiry and requested-household checks. Reuse
the tested calendar-month cap from account-session work without weakening
household binding. Prove valid app-token reads/writes and invalid, expired,
revoked, stale-version and other-household denial through HTTP. Use shared
helper evidence plus endpoint wiring checks rather than duplicate every test
for every route.

The dosage evidence map also retains foreign-medication create rejection and
wrong-household requests. Close these with focused wiring assertions alongside
the shared-auth HTTP run, using existing foreign fixture identifiers. Retain
the distinction between an invalid credential (401), an active credential for
another household (403), and a foreign resource absent from the selected
household's scope (404).

Inspect integration OAuth deliberately. Rails `BaseController#valid_session?`
accepts membership-bound OAuth grants via `active_for_membership?`; that helper
alone does not enforce scopes. Do not infer that every OAuth scope authorizes
every ordinary API operation, or copy a demonstrated scope-bypass defect.
Document the intended API credential/scoping rule before implementation and
surface any unresolved product/security choice. This finding is from source
inspection, not a demonstrated runtime exploit.

Sources: `app/controllers/api/v1/base_controller.rb`, `app/models/api_app_token.rb`,
`app/models/oauth_grant.rb`, `rust/api/src/lib.rs`, and
`rust/api/src/auth_sessions.rs`.
