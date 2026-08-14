# Production Environment Reference

This reference lists MedTracker's core production settings. Put secrets in the
deployment platform's secret store. Do not commit them or bake them into the
container image.

Optional service integrations have their own setup guides. This page covers the
application origin, database, registration policy, and process sizing.

Notification provider settings are documented in
[Notification delivery configuration](notification-delivery.md).

## Required settings

| Variable | Purpose |
| --- | --- |
| `APP_URL` | Public application origin, including the HTTPS scheme and host. Rails uses it for host checks and generated links. |
| `DATABASE_URL` | Primary PostgreSQL connection string. Production must use PostgreSQL 18. |
| `RAILS_MASTER_KEY` or `SECRET_KEY_BASE` | Rails application secret source. Use the deployment's existing encrypted-credentials or secret-key method. |

`APP_URL` must contain a host. MedTracker refuses to start in production when it
is absent or invalid.

## Database roles and connections

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_ROLE` | unset | PostgreSQL role selected after connection. Use `med_tracker_app` for hosted web and worker processes. Leave it unset for migrations unless the deployment has a separate owner role. |
| `SOLID_CACHE_DATABASE_URL` | derived from `DATABASE_URL` | Solid Cache database. |
| `SOLID_QUEUE_DATABASE_URL` | derived from `DATABASE_URL` | Solid Queue database. |
| `SOLID_CABLE_DATABASE_URL` | derived from `DATABASE_URL` | Solid Cable database. |
| `PRIMARY_DB_POOL` | `8` | Primary Active Record pool size per process. |
| `QUEUE_DB_POOL` | `8` | Solid Queue Active Record pool size per process. |

The derived database names are `medtracker_production_cache`,
`medtracker_production_queue`, and `medtracker_production_cable`. Set explicit
URLs when the deployment uses different databases or credentials.

Migration processes need the owner-capable database role. Hosted application
processes use `med_tracker_app` so forced row-level security remains effective.
See the [hosted private beta runbook](hosted-private-beta-runbook.md) before
changing these roles.

## Public origin and host checks

| Variable | Default | Purpose |
| --- | --- | --- |
| `RAILS_ALLOWED_HOSTS` | empty | Comma-separated extra hosts accepted by Rails. The host from `APP_URL` is always included. |
| `PORT` | `3000` | Puma listen port. A platform can override it. |
| `RAILS_LOG_LEVEL` | `info` | Rails log level. Do not enable verbose production logging without a privacy review. |

Only add hosts that route to this deployment. The `/up` and `/health` paths are
excluded from host authorization for platform health checks.

## Registration policy

| Variable | Default | Purpose |
| --- | --- | --- |
| `INVITE_ONLY` | database setting or automatic owner lock | Pins invitation-only registration when present. |

When `INVITE_ONLY` is absent, MedTracker uses the stored application setting. A
new database defaults to open registration. Once an active owner exists, a
database without a stored setting defaults to invitation-only registration.

Hosted deployments should set `INVITE_ONLY=true`. Do not disable it to create
another household. The missing hosted provisioning flow is tracked by issue
[#1892](https://github.com/damacus/med-tracker/issues/1892).

## Web and job sizing

| Variable | Default | Purpose |
| --- | --- | --- |
| `WEB_CONCURRENCY` | `1` | Puma worker process count. Set it only when the deployment needs two or more workers. |
| `RAILS_MAX_THREADS` | `3` | Puma threads per worker. |
| `SOLID_QUEUE_IN_PUMA` | unset | Runs the Solid Queue supervisor inside Puma when present. Use this only for a single-server layout. |
| `SOLID_QUEUE_SUPERVISOR_MODE` | `fork` | Puma Solid Queue supervisor mode. |
| `JOB_FIBERS` | `2` | Fibers used by the notifications queue worker. |

Database pools are per process. Check the total database connection budget
before increasing worker, thread, or job concurrency. A split web and worker
deployment should run Solid Queue outside Puma.

## Storage

`ACTIVE_STORAGE_SERVICE` defaults to `persistent`. A Disk-backed production
service requires a writable persistent mount at `ACTIVE_STORAGE_ROOT`, which
defaults to `/app/storage`.

For S3 and migration settings, see
[Upload storage backup and restore](upload-storage-backup-and-restore.md) and
[Home-Ops portable storage handoff](home-ops-portable-storage-handoff.md).

## Deployment check

Before release, confirm:

- required secrets come from the platform secret store;
- `APP_URL` matches the public HTTPS origin;
- web and worker processes use the expected database role;
- migrations use owner-capable credentials;
- connection pools fit the database limit;
- registration policy matches the deployment;
- storage and backup settings match the selected service.
