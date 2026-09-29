# Household invitations

Implement the five documented invitation operations: list and create household
invitations, revoke an invitation, resend it, and accept it. Write focused
black-box tests against the fixed OpenAPI schemas before production changes;
the invitations API remains a Rails reference, with existing contract tests as
prior art.

List responses contain only the seven documented invitation fields. Create
accepts only the `household_invitation` wrapper with email and member/admin role.
Verify manager authorization, household scoping, validation, active-email
duplicate handling, revocation state, and audit events. Never expose invitation
tokens or token digests in invitation summaries, acceptance responses, or audit
data.

Resend rotates an expired pending invitation, sends one replacement message,
invalidates the old token, and replays the same response without another
delivery or audit. Verify no-store headers, current manager/session checks,
accepted/revoked rejection, and the documented string invitation ID.

Acceptance requires a verified user API session matching the invitation email.
Check exact string ID response fields, no-store, one active membership on retry,
and no partial membership for expired, revoked, unknown, conflicting-identity,
or unsupported credentials. The OpenAPI contract excludes household app tokens
and delegated OAuth grants; do not preserve the existing mobile-OAuth success
expectation in old test coverage; the Rust implementation must reject it.
Acceptance retry semantics come from the invitation service, not
response-caching idempotency middleware.

The Rust sender uses `SMTP_ADDRESS`, `SMTP_PORT` (default 587), paired
`SMTP_USER_NAME` / `SMTP_PASSWORD`, `SMTP_AUTHENTICATION` (`plain` or `login`),
`SMTP_STARTTLS` (default true), `MAILER_FROM`, and `APP_URL`. Isolated acceptance
uses Mailpit with TLS explicitly disabled; it does not verify a live mail
provider. Missing or failed delivery returns 503 without committing rotation.

Reuse provisioned invitation identities and mail fixtures rather than creating
new accounts. Isolate disposable invitation rows by unique emails, and use the
selected-test compile before Sol runs HTTP RED and isolated acceptance. Do not
claim the Rust OAuth rejection as passing until its focused HTTP assertion is
green. Rails currently permits mobile OAuth acceptance; rejecting it in Rust
is an intentional correction to the documented API-session-only contract.

Use an isolated SMTP failure target to prove that failed delivery returns an
error without committing a rotated token, an idempotency receipt, or a success
audit. An ordinary keyed resend retry must deliver no second message. SMTP
acceptance and the database commit cannot be atomic across a process crash;
report that delivery window rather than claiming exactly-once delivery.
