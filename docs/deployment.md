# Deployment

MedTracker runs with Docker Compose in development, test, and production-style
setups.

## Compose files

- `docker-compose.dev.yml`: development stack
- `docker-compose.test.yml`: test stack
- `docker-compose.yml`: production-style stack

## Development deployment

Use Taskfile wrappers:

```bash
task dev:up
task dev:seed
```

Stop or inspect:

```bash
task dev:stop
task dev:logs
task dev:ps
```

## Test deployment

Start/stop test services when needed:

```bash
task test:up
task test:stop
task test:logs
```

Run full tests in the test environment:

```bash
task test
```

## Production-style compose run

If you need to run the production compose file locally:

```bash
task prod:up
```

The dedicated `migrate-prod` service completes migrations before `web-prod`
starts.

## Environment and database notes

- All environments use PostgreSQL.
- PostgreSQL version target is `18`.
- Use Rails credentials and environment variables for secrets; never commit them.
- Existing databases created before 0.5 need the
  [pre-0.5 database upgrade](pre-0-5-database-upgrade.md) bootstrap before
  running 0.5 migrations.

## Database login boundaries

Use independent secrets and URLs for every database boundary. The local Compose
passwords are development-only defaults and must not be reused in a deployment.

| Setting | Login | Session role | Used by |
|---|---|---|---|
| `BOOTSTRAP_DATABASE_URL` | Cluster administrator | None | One-time role bootstrap only |
| `MIGRATION_DATABASE_URL` | `medtracker_migration` | `med_tracker_owner` | Migrations and legacy upgrades |
| `RUNTIME_DATABASE_URL` | `medtracker_runtime` | `med_tracker_app` | Web and job processes |
| `SOLID_QUEUE_DATABASE_URL` | `medtracker_auxiliary` | None | Solid Queue |
| `SOLID_CACHE_DATABASE_URL` | `medtracker_auxiliary` | None | Solid Cache |
| `SOLID_CABLE_DATABASE_URL` | `medtracker_auxiliary` | None | Solid Cable |

Supply `MIGRATION_DATABASE_URL` as the migration container's `DATABASE_URL` with
`DATABASE_ROLE=med_tracker_owner`. Supply `RUNTIME_DATABASE_URL` as every web or
job container's `DATABASE_URL` with `DATABASE_ROLE=med_tracker_app`. Never mount
`BOOTSTRAP_DATABASE_URL` into either workload. The auxiliary URLs must use
separate databases; their login is deliberately denied access to the primary
database.

For an existing PostgreSQL cluster, stop application traffic and run the
idempotent role bootstrap from the release checkout with deployment-specific
passwords:

```bash
psql "$BOOTSTRAP_DATABASE_URL" \
  --set=runtime_password="$RUNTIME_DATABASE_PASSWORD" \
  --set=migration_password="$MIGRATION_DATABASE_PASSWORD" \
  --set=auxiliary_password="$AUXILIARY_DATABASE_PASSWORD" \
  --file compose/init-roles.sql
```

Run `db:migrate` through the migration login, then restart web and job workloads
through the runtime login. Remove the bootstrap credential from the deployment
environment after the SQL completes. The detailed legacy cutover and verification
queries are in the [pre-0.5 database upgrade](pre-0-5-database-upgrade.md) guide.

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

If either variable is absent the medicine search feature is
disabled automatically — no API calls are made.

## Flux GitOps: bootstrap first administrator

Kubernetes operators should use the dedicated runbook for complete seeding
procedures:

- [Kubernetes User Seeding Runbook](kubernetes-user-seeding.md)
- [Kubernetes NHS dm+d Release Import Runbook](kubernetes-nhs-dmd-import.md)

Quick flow selection:

| Goal                               | Command                             | Notes                                             |
|------------------------------------|-------------------------------------|---------------------------------------------------|
| Create first administrator account | `rails med_tracker:bootstrap_admin` | One-off account creation with `ADMIN_*` vars      |
| Invite initial care-team users     | `rails db:seed`                     | Reads `/app/db/seeds/users.yml`, idempotent skips |

For Kubernetes production environments managed by Flux, bootstrap the first admin
using a one-off Job manifest committed through the normal GitOps repo path.

1. Ensure the application release containing `med_tracker:bootstrap_admin` is
   deployed.
2. Add a Secret manifest (or SOPS-encrypted Secret) with:
   - `ADMIN_EMAIL`
   - `ADMIN_PASSWORD`
   - `ADMIN_NAME`
   - `ADMIN_DOB` (`YYYY-MM-DD`)
3. Add a one-off Job manifest that runs:

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: med-tracker-bootstrap-admin
spec:
  backoffLimit: 0
  ttlSecondsAfterFinished: 300
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: bootstrap-admin
          image: ghcr.io/your-org/med-tracker:<release-tag>
          command: ["bundle", "exec", "rails", "med_tracker:bootstrap_admin"]
          envFrom:
            - secretRef:
                name: med-tracker-bootstrap-admin
```

1. Commit/push the manifests and reconcile Flux for the target Kustomization.
2. Verify completion:

```bash
kubectl get jobs -n <namespace>
kubectl logs job/med-tracker-bootstrap-admin -n <namespace>
```

1. Confirm the admin can sign in and access `/admin`.
2. Remove/disable bootstrap manifests in Git and reconcile Flux again.

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
