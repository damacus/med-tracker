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
runners. Dispatch it **from main** with **publish** enabled to publish a
multi-platform `ghcr.io/damacus/med-tracker-loco:sha-<commit>` image after both
checks pass. Publication verifies checksums, image IDs and architectures of the
saved tested images, pushes those images, and joins their registry digests into
the multi-platform manifest. The publication job has no source checkout, build
or application execution. Pull requests and dispatches from other branches have
no package write permission and cannot publish. Transferred image archives
expire after one day.
The existing Rails image publisher remains separate. Publishing an image does
not deploy it.

`task image:check` compares the image's pinned Rust compiler with the compiler
selected by the application's stable CI policy, its Node version with `mise.toml`
and the exact Node patch selected by app CI, and its Task version with app CI.
The local check compares Node majors; CI additionally compares the resolved patch.
The image uses Rust 1.99.1 and Node 24.22.1;
`Cargo.toml`'s `rust-version` remains the minimum supported compiler. A new stable
release makes the alignment check fail until the Rust image version and verified
digest are updated. Node 24 and Task 3.52.0 match the application's selections.
The build frontend, base images, BuildKit daemon and workflow actions are pinned
by digest or commit. CI verifies Task 3.52.0 and Buildx 0.38.1 binaries against
committed checksums before running them. BuildKit 0.34.1 runs without the default
host-network and insecure-security entitlements.
Cargo and npm use their lockfiles; npm dependency install scripts are disabled.
Build caches are separated by architecture and contain no deployment credentials.
The smoke checks use a read-only filesystem with all capabilities dropped and
privilege escalation disabled. They pin their disposable PostgreSQL image too.

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

Configure the container platform to probe the server's HTTP `/health` endpoint
on port 5150 and supervise the worker and scheduler processes separately. The
scratch image has no embedded Docker `HEALTHCHECK`: executing an HTTP probe
inside it would require adding another executable. Trivy reports this as a low
severity configuration finding; platform probes provide the health check.
