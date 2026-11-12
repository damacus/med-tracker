# Install MedTracker

MedTracker 0.6 runs on PostgreSQL 18 with a fresh database and new accounts.
One installation serves multiple accounts and households, with application
permissions and PostgreSQL row-level security separating their data. Keep any
previous Rails database separately: setup does not import, reset or adopt it.

Docker Compose and Helm use the same application binary. Initial setup creates
the schema, internal roles and restricted application access. Routine upgrades
run automatically with a separate non-superuser schema-update login. Operators
do not create roles, assign grants or maintain database setup scripts.

Use a released image and BJW's app-template chart, preferably pinning the image digest.
Expose the application through HTTPS and keep secure production cookies enabled.
See [release notes](../releases/0.6.md) for remaining limitations and
[configuration](../../config/production.yaml) for all available settings.

## PostgreSQL connection fields

MedTracker accepts the standard `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER` and
`PGPASSWORD` fields. Supply host, database, username and password together;
the port defaults to 5432. A partial field configuration stops startup with a
configuration error. Passwords and database names are encoded by the application,
so Kubernetes manifests do not need to construct or escape connection URLs.
Login names retain MedTracker's existing restriction to ASCII letters, digits,
underscores and hyphens.

With CloudNativePG, read `host`, `port` and `dbname` from its generated application
Secret. Read `username` and `password` from the credential Secret for the process
being started. The web, worker and scheduler use the restricted runtime login;
the migration Job uses the schema-update login. Do not give those processes the
database-owner or administrator credential from CNPG's generated Secrets.

For initial setup, use the same endpoint fields with the administrator's
`PGUSER` and `PGPASSWORD`. Supply `MEDTRACKER_RUNTIME_PGUSER` and
`MEDTRACKER_RUNTIME_PGPASSWORD` for the runtime login, and
`MEDTRACKER_UPDATE_PGUSER` and `MEDTRACKER_UPDATE_PGPASSWORD` for schema updates.
Each pair must be complete and all three logins must be distinct. The existing
setup checks still require an empty or recognised database and restricted roles.

Existing `DATABASE_URL` configurations and saved Loco configuration keep working.
An explicit `DATABASE_URL` takes precedence over the active connection fields.
The corresponding `MEDTRACKER_RUNTIME_DATABASE_URL` and
`MEDTRACKER_UPDATE_DATABASE_URL` settings also take precedence over their
credential fields. Without either supplied setup credential, native setup keeps
generating and storing separate credentials as before.

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

Use upstream [BJW app-template](https://bjw-s-labs.github.io/helm-charts/docs/app-template/)
5.3.0 directly. MedTracker supplies ordinary values files, with no custom chart,
templates or chart archive to maintain. Check out the application release and
copy [the runtime values](https://github.com/damacus/med-tracker/blob/main/deploy/helm/values.yaml) to your own `values.yaml`.
Set both public-origin variables under `controllers.web.containers.app.env` to
your HTTPS origin. YAML anchors share these settings with the worker and scheduler.
Configure your existing ingress or Gateway to the web Service on port 5150.

Provision a fresh PostgreSQL 18 database and these Secrets through your existing
secret manager. For example, HomeOps uses 1Password and ExternalSecrets.

| Secret | Required keys | Used by |
| --- | --- | --- |
| `med-tracker-bootstrap` | `DATABASE_URL` for the empty database's administrator | Initial setup only |
| `med-tracker-runtime` | `DATABASE_URL`, `AUTH_SESSION_SECRET`, `MEDTRACKER_SESSION_KEY` | Web, worker and scheduler |
| `med-tracker-update` | `DATABASE_URL` for a separate schema-update login | Automatic schema updates |
| `med-tracker-integrations` | SMTP variables above, plus any enabled storage and `OIDC_*` settings | Web, worker and scheduler |

Choose distinct runtime and update usernames and strong URL-encoded passwords;
all three connection URIs must identify the same database. Loco's native setup
task creates these logins, grants and RLS. You do not create roles or write SQL.
Use a persistent random session secret and a base64-encoded 64-byte random session
key. Keep these credentials stable and back them up with the database. The values
files reference Secrets; they neither generate nor manage their lifecycle.

Install with [the initial-setup overlay](https://github.com/damacus/med-tracker/blob/main/deploy/helm/setup.values.yaml):

```fish
helm install med-tracker oci://ghcr.io/bjw-s-labs/helm/app-template --version 5.3.0 --values values.yaml --values deploy/helm/setup.values.yaml --wait --timeout 10m
```

The setup hook runs `task database-setup` with administrator access, followed by
`db migrate` with the non-superuser update login. Failed hooks stop the rollout.
The web, worker and scheduler receive only runtime credentials and integrations.
The setup overlay also supports an existing release pointed at a new empty
database; it does not import or reset the old database.

For routine upgrades, update the image tag or digest in your runtime values and
omit the setup overlay. Reset stored Helm values so the initial setup controller
is removed, then reapply your complete runtime values file:

```fish
helm upgrade med-tracker oci://ghcr.io/bjw-s-labs/helm/app-template --version 5.3.0 --reset-values --values values.yaml --wait --timeout 10m
```

Do not use `--reuse-values`: it would retain the privileged initial setup
controller. The routine values contain only the restricted migration hook, which
runs before each upgrade. Ordinary upgrades and Pod restarts need no administrator
Secret. With Flux, use the same upstream chart and values and remove the setup
controller after successful initial setup. The chart version is independent of
the application image version; existing HomeOps deployments already use this chart.

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
