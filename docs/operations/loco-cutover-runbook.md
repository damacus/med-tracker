# Install MedTracker

MedTracker 0.6 runs on PostgreSQL 18 with a fresh database and new accounts.
One installation serves multiple accounts and households, with application
permissions and PostgreSQL row-level security separating their data. Keep any
previous Rails database separately: setup does not import, reset or adopt it.

Docker Compose and Helm use the same application binary. Initial setup creates
the schema, internal roles and restricted application access. Routine upgrades
run automatically with a separate non-superuser schema-update login. Operators
do not create roles, assign grants or maintain database setup scripts.

Use a released image and chart version, preferably pinning the image digest.
Expose the application through HTTPS and keep secure production cookies enabled.
See [release notes](../releases/0.6.md) for remaining limitations and
[configuration](../../config/production.yaml) for all available settings.

## Docker Compose

Check out the release you want to install. Create a private `.env` file
(`chmod 600 .env`) with a strong PostgreSQL bootstrap password, the image
reference, the public hostname and your mail configuration:

```dotenv
POSTGRES_PASSWORD=<long-random-password>
LOCO_IMAGE=ghcr.io/damacus/med-tracker-loco:0.6.0
MEDTRACKER_PUBLIC_HOST=https://med-tracker.example.org
MEDTRACKER_COOKIE_SECURE=true
MEDTRACKER_SMTP_ENABLED=true
MEDTRACKER_SMTP_HOST=smtp.example.org
MEDTRACKER_SMTP_USER=<smtp-user>
MEDTRACKER_SMTP_PASSWORD=<smtp-password>
MEDTRACKER_MAIL_FROM=MedTracker <notifications@example.org>
```

The default bootstrap connection uses the PostgreSQL service in Compose. Use a
URL-safe bootstrap password, or set `MEDTRACKER_SETUP_DATABASE_URL` to the full
connection URI with a percent-encoded password. Bootstrap access goes only to
PostgreSQL and the initial setup service.

Run initial setup, then start the application:

```fish
docker compose -f compose.yaml -f compose.production.yaml --profile setup run --rm setup
docker compose -f compose.yaml -f compose.production.yaml up -d
```

Setup generates runtime and schema-update credentials and persistent session
keys. To choose credentials instead, supply `MEDTRACKER_RUNTIME_DATABASE_URL`
and `MEDTRACKER_UPDATE_DATABASE_URL` before setup. Both must point to the same
database as the setup connection. Existing passwords are never silently changed.

Generated configuration is stored in separate private named volumes using
Loco's native YAML overlays. The running app, worker and scheduler mount only
runtime configuration, read-only. The update service mounts only update
configuration. The current production template is supplied as a Compose config,
so mail and other environment settings remain configurable after installation.

The web service listens on loopback port 5150. Connect your HTTPS reverse proxy
to it. Set `MEDTRACKER_PORT` if that local port is occupied.

For a routine upgrade, update `LOCO_IMAGE` to the desired release and run:

```fish
docker compose -f compose.yaml -f compose.production.yaml pull
docker compose -f compose.yaml -f compose.production.yaml up -d
```

Compose runs the schema-update service before starting the new application
processes. A failed update blocks their startup. Ordinary restarts reuse the
same credentials and data; do not rerun initial administrator setup for each
release. PostgreSQL data and both configuration volumes must be backed up.
Do not remove volumes as part of an upgrade.

## Helm

Provision a fresh PostgreSQL 18 database. Put its administrator connection URI
in a Kubernetes Secret using your existing secret manager; the default key is
`DATABASE_URL`. For example, HomeOps uses 1Password and ExternalSecrets.
The application only needs administrator access during initial installation.

Create a values file:

```yaml
database:
  host: postgres.example.internal
  name: medtracker
  bootstrap:
    existingSecret: med-tracker-bootstrap
env:
  MEDTRACKER_PUBLIC_HOST: https://med-tracker.example.org
  MEDTRACKER_PUBLIC_ORIGIN: https://med-tracker.example.org
envFrom:
  - secretRef:
      name: med-tracker-integrations
```

The integrations Secret supplies the SMTP variables shown above and any enabled
storage or identity-provider configuration. Configure HTTPS ingress or Gateway
API routing to the chart's Service.

```fish
helm install med-tracker https://github.com/damacus/med-tracker/releases/download/v0.6.0/med-tracker-0.6.0.tgz --values values.yaml --wait --timeout 10m
```

The chart generates runtime and schema-update Secrets, runs initial setup before
application startup, and retains those credentials on subsequent upgrades.
To use an existing secret manager for all credentials, supply
`database.runtime.existingSecret` and `database.update.existingSecret`.
Both contain `DATABASE_URL`; the runtime Secret also supplies
`AUTH_SESSION_SECRET` and `MEDTRACKER_SESSION_KEY`.

Each GitHub release includes the chart archive and its SHA-256 checksum.
Upgrade through Helm, or let Flux reconcile the chart from the matching Git tag:

```fish
helm upgrade med-tracker https://github.com/damacus/med-tracker/releases/download/v<next-version>/med-tracker-<next-version>.tgz --reuse-values --wait --timeout 10m
```

The chart runs the native `db migrate` command in a pre-upgrade hook using the
schema-update login. A failed hook stops the rollout. Routine upgrades do not
mount or require the administrator Secret. If image tags or digests are
explicitly pinned in values, update them to match the chart release.

When an existing Helm release is being pointed at a new empty database, use
`setup.bootstrapOnUpgrade: true` for that one cutover. Remove that setting after
successful initial setup. This does not migrate data from the old database.

See the [chart reference](https://github.com/damacus/med-tracker/blob/main/charts/med-tracker/README.md) for existing-secret,
routing and image settings. Generated Secrets are retained on uninstall; keep
them with database backups rather than deleting them during an upgrade.

## Configuration and schema changes

The server, worker and single scheduler use restricted runtime credentials.
The runtime `auto_migrate` setting remains false because Compose and Helm perform
automatic schema updates with separate credentials. Truncation and database
recreation remain disabled.

Initial setup establishes RLS and grants. Routine 0.6 releases preserve that
security model and remain compatible with the previous running application.
Deliberate RLS or privilege changes belong in a planned release such as 0.7
with a one-time upgrade guide.

SMTP is required for real account verification and recovery mail. Configure the
private S3-compatible storage settings for enabled avatar and dm+d features.
Keep session keys stable across replicas and restarts; never substitute the
committed development fixtures. Production seeding is not an installation step.

Supported encrypted portable import/export remains subject to the
[portable-data contract](../api/portable-data.md). It is not a general Rails
database or credential migration mechanism.
