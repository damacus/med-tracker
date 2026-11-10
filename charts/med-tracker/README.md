# MedTracker Helm chart

See the [installation guide](../../docs/operations/loco-cutover-runbook.md)
for PostgreSQL 18 requirements, credentials and upgrades.

Set `database.host` and `database.bootstrap.existingSecret`. The bootstrap Secret
contains `DATABASE_URL` for the empty database's administrator. The chart
generates separate runtime and schema-update credentials, retaining them across
upgrades. Alternatively, provide `database.runtime.existingSecret` and
`database.update.existingSecret`; both default to the `DATABASE_URL` key. Runtime
Secrets also provide `AUTH_SESSION_SECRET` and `MEDTRACKER_SESSION_KEY`.

Initial setup runs as a pre-install hook. Routine schema updates run as a
pre-upgrade hook with a non-superuser login. The server, worker and one scheduler
only receive runtime credentials. Credential Secrets run before database hooks;
failed hooks stop installation or upgrade. Keep ordinary schema updates backward
compatible with the previous running version.

When reusing an existing Helm release with a new empty database, set
`setup.bootstrapOnUpgrade: true` for the first cutover and remove it after
successful setup. Ordinary upgrades do not need the bootstrap Secret.

Use `env` and `envFrom` for application configuration and integration Secrets.
Use `route` for Gateway API HTTPRoutes, or expose the Service through your own
HTTPS ingress. `service.port` defaults to 5150. `fullnameOverride` preserves an
existing Service and server Deployment name. `image.digest` pins the image.

Generated credential Secrets are retained when the release is uninstalled. Keep
them with database backups; deleting them is an explicit credential-lifecycle
operation. Reusing an existing database with unrelated generated credentials
will fail rather than silently resetting its passwords.
