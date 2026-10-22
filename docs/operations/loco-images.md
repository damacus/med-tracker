# Root Loco Linux images

Build the root application with `task image:build`, then run `task image:test`.
Both tasks accept `LOCO_IMAGE` (default `medtracker-loco:local`). Docker is required;
the build uses the committed Cargo and npm lockfiles. The checks create and remove
their own PostgreSQL container, migrate its empty database, and boot the server
and worker with a restricted runtime login. No existing database is used.
The final scratch image contains the Linux executable, its shared libraries,
TLS roots, timezone data,
production configuration, Tera templates, static assets and PDF fonts. It runs as
UID/GID 65532, without a shell or compiler.

The **Loco Linux images** workflow builds and checks amd64 and arm64 on native
runners. Dispatch it with **publish** enabled to publish a multi-platform
`ghcr.io/damacus/med-tracker-loco:sha-<commit>` image after both checks pass.
The existing Rails image publisher remains separate. Publishing an image does
not deploy it.

Use the same image with these commands (arguments follow the image name):

| Process | Arguments | Database identity |
| --- | --- | --- |
| Server (default) | `start --environment production` | Restricted runtime role |
| Worker | `start --worker --environment production` | Restricted runtime role |
| Scheduler (one instance) | `scheduler --environment production` | Restricted runtime role |
| Schema setup (separate job) | `db migrate --environment production` | Migration role |

The server binds to `0.0.0.0:5150`. Supply `DATABASE_URL`,
`MEDTRACKER_PUBLIC_HOST`, `AUTH_SESSION_SECRET` and `MEDTRACKER_SESSION_KEY` through
the deployment's secret/configuration mechanism. All other production settings
remain in `config/production.yaml`. Schema migration is disabled at application
startup. Follow the [fresh rollout guidance](loco-cutover-runbook.md) for the
fresh database and credential boundary.

Workers and servers need the same configured private S3 storage. Credentials and
uploaded files are never built into the image. With a read-only root filesystem,
mount a writable temporary directory at `/tmp` for UID 65532. No local persistent
volume is needed for configured object storage. Mount an alternate production
configuration at `/app/config/production.yaml` if the deployment needs settings
not exposed by the packaged configuration. Run a single scheduler independently
of worker replicas; do not also enable `--scheduler` on those workers.
