# Deployment

The published product image remains Rails while the Loco migration is in
progress. Root `task release-image` is gated until complete migration acceptance.
The Rails rollback uses profiles in `rails/compose.yaml` for development,
testing, and local validation of the production image.

## Loco authentication secrets

Existing authenticator enrolments require `RAILS_SECRET_KEY_BASE` to contain the
same secret used by the Rails application when those credentials were created.
Rodauth derives the authenticator secret from the stored key and Rails
`secret_key_base`; a new browser-session key cannot replace it. Supply this value
through the deployment's secret store, never a committed environment file. A
missing or empty value prevents TOTP completion rather than falling back to the
raw stored key.

If the Rails installation used an old HMAC secret during rotation, preserve it
explicitly as `RAILS_OLD_SECRET_KEY_BASE` alongside the current secret. Do not
generate a new value for either setting during a restart. `MEDTRACKER_SESSION_KEY`
remains the separate persistent 64-byte, Base64-encoded browser-session key.

Password verification alone does not create a clinical session for a TOTP account.
The additional-factor challenge expires after five minutes and requires a new
password verification after expiry. Existing authenticator credentials stay in
`account_otp_keys`; no re-enrolment or schema change is required by this flow.

## Loco passkeys

Configure the server's canonical public origin before registering passkeys. The
relying-party ID comes from that origin's hostname; verification requires the
configured scheme, hostname and port. Use a DNS hostname, or `localhost` for the
owned browser fixtures. Native browsers reject an IP literal as a relying-party
ID. Existing credentials remain bound to their original relying party.

The adapter retains Rodauth's Base64url COSE public keys, credential IDs, user
handles and signature counters in the existing WebAuthn tables. Ceremony state
comes from webauthn-rs-core and is held in the encrypted browser session. A
password-verified challenge keeps its original authentication time and expires
after five minutes.

The retained schema has no historical backup-eligibility or backup-state fields.
The adapter therefore uses the maintained authenticator-data parser's observed
flags, followed by full library signature verification. It cannot detect changes
against historical backup flags or infer that a retained credential is bound to
physical hardware. Counter, user-verification, challenge, origin and account
checks still apply. The browser client uses the standard
`PublicKeyCredential` JSON conversion methods; browsers without those methods
cannot complete this passkey flow.

### Approved cutover decision: unsupported passkeys

**Decision approved by the project owner on 6 October 2026:** Loco will drop support
for passkey algorithms that its maintained WebAuthn library cannot verify.
Rails' default algorithm list includes PS256, which the selected Rust library
does not support. We will accept this authentication compatibility break rather
than add custom verification code or another identity provider.
The pinned library accepts ES256 and RS256, with RSA restricted to a 2048-bit
modulus and a three-byte exponent. Other unsupported key shapes follow the same
replacement decision; an algorithm name alone does not establish compatibility.

Affected users must reauthenticate through a supported sign-in/recovery flow and
register a replacement passkey. Their unsupported passkey will not work in Loco.
An unsupported key must not prevent another supported key from working. Security
settings must explain which keys need replacing; MFA must not be silently bypassed.
Do not delete unsupported credential rows as part of the migration; retain them
for Rails rollback. Normal authenticated removal remains an explicit user action.

The owner accepts invalidating unsupported historical passkeys and expects at most
one affected account. Counting affected production accounts is not a migration or
cutover gate. Verify recovery and replacement with synthetic unsupported-only and
mixed-key accounts, and include the transition in the user instructions. This
decision does not permit an authentication or MFA bypass.

## Approved cutover and rollback design

On 6 October 2026, the owner approved rollback to the independently preserved
pre-cutover database state. All data created during the Loco period may be dropped
or ignored if rollback is needed. Rails compatibility with Loco-created data and
lossless post-cutover rollback are not required.

Before switching, save a production database dump or preserve a replica. The saved
state must be independent of subsequent Loco writes. Stop replication before Loco
starts writing if the rollback copy is a replica. Prove restoration and Rails
operation on representative saved data during the disposable rehearsal; retain
the exact Rails image/configuration and restore procedure for the authorised switch.

Scale Rails servers to zero before Loco starts. Stop separately deployed Rails
workers and schedulers too, prevent new scheduled jobs, and verify that no Rails
process can continue writing. Only the intended application may own writes.
If rollback is required, stop Loco servers, workers and schedulers first, restore
the saved pre-cutover state, then turn Rails back on against that state.

Loss of pending or failed Rails jobs is accepted. Do not transfer the old queue or
reconcile every old delivery as a cutover prerequisite. Correct Loco jobs,
permissions, retries, duplicate suppression, restart recovery and shutdown behaviour
remain required.

Invalidate existing sessions and tokens at cutover; historical continuity is not
required. Prove the invalidation mechanism on synthetic credentials, while retaining
account credentials for fresh authentication and enforcing MFA. Previously issued
download links may expire. Preserve underlying files and data and issue new authorised
links. Historical system export formats and pre-cutover offline queues need not be
carried over; new exports and future offline replay still require working Loco flows.

Browser pages use daisyUI and clear Loco routes. Keep the same colour schemes,
required information, actions and accessibility; exact Rails pixels, CSS values,
theme exports and historical browser URLs are not acceptance requirements. Users
may enable push notifications again; new subscriptions, preferences and delivery
must work. PDF reports must match the existing appearance. Inspect the current
renderer, fonts and retained Rust implementation before selecting a renderer, and
verify it inside the final scratch image.

FHIR/SMART, MCP, AI and external medication lookup are deferred from the first
production release. They remain part of the complete migration objective. Exact
AI/lookup provider configuration parity is unnecessary. Core medication entry and
care workflows remain required. Report core production readiness separately from
completion of every migration capability.

The subsequent first-release scope decision requires automated NHS dm+d import/
reconciliation and scanner, medication reviews with background refresh, complete
platform administration, time-limited support access, household export/closure/
retention holds/permanent deletion, avatar uploads, complete device/session management
and automatic live dose/stock updates. Catalogue/scanner and reviews remain core
despite optional AI/provider lookup deferral. First production does not require
browser offline capture, portable imports or acceptance of both native apps. These
remain in the full migration goal; public API correctness stays required.

These are approved design and acceptance decisions. They do not authorise stopping
production, copying live data, deleting data or deploying now. The saved-state
rehearsal and the complete Loco release still need evidence before live approval.

## Invitation email transactions

Invitation delivery uses Loco's standard `MailerWorker` and `Email` payload.
Loco 1.2's PostgreSQL enqueue API takes a separate pool; it cannot join the existing
SeaORM tenant transaction. The invitation adapter inserts that standard job into
the owner-provisioned `pg_loco_queue` within the same transaction as the invitation,
token rotation and audit changes. Loco owns job execution and retries.

Before accepting this delivery, prove that an enqueue failure rolls back the
invitation changes and that the registered worker consumes a committed job and
delivers its email. Browser journeys use an isolated SMTP inbox to verify delivery
and the actual acceptance link. A queue row alone is not delivery evidence.

## Registration policy database access

The migration adds a SELECT policy for active-owner memberships to the existing
database owner role. Its fixed-search-path function
`public.registration_has_active_owner()` returns only a boolean to the application.
The application retains tenant row filtering, cannot assume the owner role and
receives no new table or schema permissions. Public function execution is revoked.
The owner role remains subject to forced row-level security; the additional policy
allows this bounded lookup. No new database role or operator bootstrap is needed.

Preserve the retained registration setting: `INVITE_ONLY` takes precedence;
otherwise initialise a missing stored setting from active-owner existence.
A stored false setting stays open after the first household is created. Neither
these migration files nor this documentation authorise a live database change.

## Compose profiles

- `dev`: development stack
- `test`: test stack
- `prod`: local validation of the production image

## Development deployment

Use Taskfile wrappers:

```fish
task rails:dev:portless
task rails:dev:seed
```

Stop or inspect:

```fish
task rails:dev:stop
task rails:dev:logs
task rails:dev:ps
```

## Test deployment

Start/stop test services when needed:

```fish
task rails:test:up
task rails:test:stop
task rails:test:logs
```

Run full tests in the test environment:

```fish
task rails:test
```

## Local production-image validation

The `prod` profile builds the production image and migrates its local PostgreSQL
database before starting the application. It is not a real production
deployment.

```fish
task rails:prod:build
task rails:prod:up
task rails:prod:ps
```

`task rails:prod:up` runs the `migrate-prod` service before starting the web service.
Use the Task wrappers to inspect and stop the local stack:

```fish
task rails:prod:logs
task rails:prod:stop
```

For a real reachable deployment, follow the
[hosted private beta runbook](operations/hosted-private-beta-runbook.md) or the
Kubernetes runbooks linked below.

## Environment and database notes

See the [production environment reference](operations/production-environment.md)
for required settings, database roles, registration policy, and process sizing.

- All environments use PostgreSQL.
- PostgreSQL version target is `18`.
- Use Rails credentials and environment variables for secrets. Never commit
  them.
- Existing databases created before 0.5 need the
  [pre-0.5 database upgrade](pre-0-5-database-upgrade.md) bootstrap before
  running 0.5 migrations.
- Leave `DATABASE_ROLE` unset for migrations in existing shared-login
  deployments. The web process uses the same database login and may set
  `DATABASE_ROLE=med_tracker_app` for runtime row-level security. Owner-role
  switching is deferred until there is an explicit ownership-adoption and
  rollback design for existing databases.

## External API credentials

See [optional integration configuration](operations/optional-integrations.md)
for the complete service inventory and privacy boundaries.

### NHS dm+d medicine search

The medicine search feature requires a system-to-system account
from the NHS England Terminology Server. See
[NHS dm+d Integration](nhs-dmd-integration.md) for the full
setup guide including how to request credentials.

| Variable                | Required | Description                   |
|-------------------------|----------|-------------------------------|
| `NHS_DMD_CLIENT_ID`     | Yes      | OAuth2 client ID from NHS     |
| `NHS_DMD_CLIENT_SECRET` | Yes      | OAuth2 client secret from NHS |

If either variable is absent, the medicine search feature is disabled and does
not make NHS API calls.

## Bootstrap the first administrator

Kubernetes operators should use the dedicated runbook for complete seeding
procedures:

- [Kubernetes User Seeding Runbook](kubernetes-user-seeding.md)
- [Kubernetes NHS dm+d Release Import Runbook](kubernetes-nhs-dmd-import.md)

Use the runbook for both Kubernetes Jobs. It includes the required runtime
settings, household selector, supported invitation roles, verification, and
cleanup steps.

Quick flow selection:

| Goal                               | Command                             | Notes                                             |
|------------------------------------|-------------------------------------|---------------------------------------------------|
| Create first administrator account | `rails med_tracker:bootstrap_admin` | One-off account creation with `ADMIN_*` vars      |
| Invite initial care-team users     | `rails db:seed`                     | Reads `/app/db/seeds/users.yml`, idempotent skips |

After the first admin exists, self-registration without invitations is blocked.

## Rebuild environments

Development rebuild (destructive to dev volumes):

```bash
task rails:dev:rebuild
```

Test rebuild (destructive to test volumes):

```bash
task rails:test:rebuild
```
