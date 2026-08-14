# Deployment

MedTracker uses profiles in a single `compose.yaml` file for development,
testing, and local validation of the production image.

## Compose profiles

- `dev`: development stack
- `test`: test stack
- `prod`: local validation of the production image

## Development deployment

Use Taskfile wrappers:

```fish
task dev:portless
task dev:seed
```

Stop or inspect:

```fish
task dev:stop
task dev:logs
task dev:ps
```

## Test deployment

Start/stop test services when needed:

```fish
task test:up
task test:stop
task test:logs
```

Run full tests in the test environment:

```fish
task test
```

## Local production-image validation

The `prod` profile builds the production image and migrates its local PostgreSQL
database before starting the application. It is not a real production
deployment.

```fish
task prod:build
task prod:up
task prod:ps
```

`task prod:up` runs the `migrate-prod` service before starting the web service.
Use the Task wrappers to inspect and stop the local stack:

```fish
task prod:logs
task prod:stop
```

For a real reachable deployment, follow the
[hosted private beta runbook](operations/hosted-private-beta-runbook.md) or the
Kubernetes runbooks linked below.

## Environment and database notes

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
task dev:rebuild
```

Test rebuild (destructive to test volumes):

```bash
task test:rebuild
```
