# Deployment

The Loco Linux images workflow builds, checks and publishes the Rust image on
every push to main as `ghcr.io/damacus/med-tracker-loco:sha-<full commit>`.
Only the current main revision can update `beta`; reruns of older commits retain
their own image without changing beta. Its run summary records the source commit and immutable digest
to deploy. Manual publication from main remains available. The existing Rails
publisher remains separate.
Root Loco is the fresh Rust application; use the [fresh Rust rollout notes](operations/loco-cutover-runbook.md)
for its image, configuration, database and recovery guidance. There is no separate
rehearsal or runtime evidence dossier before the owner's normal version upgrade.

The Rails Compose and operator commands below remain reference material. They
are not the Rust rollout procedure.

There is no supported rollback from 0.6 to Rails. A full Rails export at the
transition is an operator action, not an automatic import or a claim that an
export has already happened. Any retained Rails site contains only its
pre-cutover state. Preserve data written in 0.6 with a separate Rust-side export;
returning to the old site would not carry those records back. These notes do
not promise that Rails can import that export or provide a lossless switchback.

## Loco authentication secrets

Loco uses Better Auth with a persistent 64-byte, Base64-encoded
`MEDTRACKER_SESSION_KEY`, provisioned through the deployment secret store.
Development uses the committed synthetic browser-session key when the environment
variable is absent. Do not use that development key in production.

Loco uses Better Auth’s native schema, store and authentication plugins. It starts
with fresh authentication data: Rails passwords, authenticator enrolments,
passkeys, sessions and recovery tokens are not imported. Schema setup creates the
native tables directly, without transitional credential tables or auth-data
conversion. Use a new empty database; setup does not reset or adopt a populated one.
Provisioning or replacing any populated database requires separate approval.

Users enrol native credentials and use native MFA backup codes or email/password
recovery. A backup code completes the second sign-in step and cannot sign in
alone. Passkey-only accounts add a password before enabling the native TOTP
plugin. New authenticator enrolments require verification before activation.

Loco authentication does not need Rails secrets. Keep retained Rails reference
credentials and secrets separate from the Rust deployment. See the
[authentication guide](two-factor-authentication.md) for native account flows.

Production browser, CSRF and Better Auth cookies share the validated secure-cookie
policy. `MEDTRACKER_COOKIE_SECURE=false` is rejected in production; TLS termination
at a proxy does not make authentication cookies non-Secure. Set the canonical
public scheme/hostname through `MEDTRACKER_PUBLIC_HOST` and ensure the configured
Loco port matches the public origin used for generated links and relying-party
verification. Internal HTTP alone does not establish the public origin.

## Loco passkeys

Configure the server's canonical public origin before registering passkeys. The
relying-party ID comes from that origin's hostname; verification requires the
configured scheme, hostname and port. Use a DNS hostname, or `localhost` for the
owned browser fixtures. Native browsers reject an IP literal as a relying-party
ID.

Better Auth’s native passkey plugin owns registration, verification and storage.
Users register new Loco passkeys. Historical Rails credentials remain separate
reference data. The browser
client uses standard `PublicKeyCredential` JSON conversion methods; browsers
without those methods cannot complete this flow.

## Fresh Rust release scope

[ADR 0016](adrs/0016-fresh-rust-without-legacy-compatibility.md) supersedes the
earlier populated-schema adoption, credential migration and saved-state Rails
rollback design. Start with a new empty PostgreSQL 18 database and fresh Rust
credentials. Historical passkeys are not imported, regardless of algorithm.
Do not reset existing databases or assume Rails can consume Rust-created data.

Use the [fresh Rust rollout notes](operations/loco-cutover-runbook.md) for the
normal image/version upgrade, writer ownership, configuration and stop/recovery
boundaries. The owner observes runtime behaviour during release; no standalone
rehearsal, final evidence dossier or duplicate application-acceptance project is
required. Keep normal tests/CI for changed deliverables and fix concrete defects.
Deployment remains a separate owner action.

The owner has accepted the delivered Slice 7 worker/notification scope and waived
extra process stop/restart and SIGTERM rehearsals. Native/offline work, new export
products, canary jobs and optional integrations retain their explicit deferrals.
Physical notification checks remain deferred until after deployment. These
decisions do not claim inbox receipt, device display or unperformed checks.

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
