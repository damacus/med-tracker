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

Before cutover, identify affected accounts and verify the recovery and replacement
journey, including accounts whose only passkey is unsupported. Include this
transition in the user-facing migration instructions and the populated cutover
rehearsal. Approval of the decision does not mean these implementation checks have
passed; they remain release requirements.

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
