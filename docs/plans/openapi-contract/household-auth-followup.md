# Household credential prerequisite

The shared credential gap is implemented and independently reviewed on
26 September 2026. `rust/api/src/lib.rs#authenticate` now accepts household
app tokens alongside API sessions and mobile OAuth. The final dosage HTTP run
passed 13/13 groups (`mtcontract-2407927431414afe`); account-session regression
passed 8/8 (`mtcontract-df1315fee61146cb`).

Preserve the existing active-account/user, lockout, membership, operational
household, permissions-version, expiry and requested-household checks. Reuse
the tested calendar-month cap from account-session work without weakening
household binding. Prove valid app-token reads/writes and invalid, expired,
revoked, stale-version and other-household denial through HTTP. Use shared
helper evidence plus endpoint wiring checks rather than duplicate every test
for every route.

The dosage HTTP run also proves foreign-medication create rejection and
wrong-household requests across all five methods. Retain
the distinction between an invalid credential (401), an active credential for
another household (403), and a foreign resource absent from the selected
household's scope (404).

Operational issuing-household rejection is now proven by the same-token
200/401/200 test through active, archived and restored household states.
The final 14/14 dosage run closes this prerequisite; see `coverage/report.md`.

Integration scope intent is resolved by existing source: `OauthApplication`
limits integration clients to SMART read scopes; `medtracker` is mobile-only.
`docs/api/smart-on-fhir.md` explicitly separates SMART from first-party `/api/v1`.
Keep ordinary API integration-token denial and test a legitimate SMART scope.
Rails `BaseController#valid_session?` accepts membership-bound OAuth grants via
`active_for_membership?`, which alone does not enforce scopes. Do not port that
potential scope bypass. This is a source finding, not a demonstrated runtime
exploit. Account-level logout may still revoke a known SMART credential.

Sources: `app/controllers/api/v1/base_controller.rb`, `app/models/api_app_token.rb`,
`app/models/oauth_grant.rb`, `rust/api/src/lib.rs`, and
`rust/api/src/auth_sessions.rs`.
