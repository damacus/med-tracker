# Root Loco Linux images

Build the root application with `task image:build`, then run `task image:test`.
Both tasks accept `LOCO_IMAGE` (default `medtracker-loco:local`). Docker is required.
The build uses the committed Cargo and npm lockfiles and accepts `CARGO_BUILD_JOBS`
(default `2`, use `1` on a constrained VM). The checks create and remove
their own PostgreSQL container, migrate its empty database, and boot the server
and worker with a restricted runtime login. No existing database is used.
The final scratch image contains the Linux executable, its shared libraries,
TLS roots, timezone data,
production configuration, Tera templates, static assets and PDF fonts. It runs as
UID/GID 65532, without a shell or compiler.

Native scheduler execution is covered separately by
`task test:scheduler:process` and `task test:scheduler:image LOCO_IMAGE=<image>`.
The image acceptance creates a private database and restricted login, expires a
support session, then inserts another session and waits for a later cron tick.
It checks both attributable audit records, absence of duplicate expiry, UID 65532,
missing `/bin/sh` and clean scheduler shutdown. These checks execute a schedule;
listing configured jobs alone does not prove execution.

On a Docker host with exhausted default network pools, the image checks accept
`LOCO_IMAGE_SUBNET=<unused-private-subnet>`. Choose an unused subnet; the checks
remove only the network and containers they created.

Loco's released 1.2.0 scheduler requires a correction for direct native-task
execution. The owner approved a temporary exception to the registry-only dependency
rule: `Cargo.toml` patches `loco-rs` to
[`damacus/loco` commit `5224bc58f9292a2f4713ee333bded785e5266f09`](https://github.com/damacus/loco/commit/5224bc58f9292a2f4713ee333bded785e5266f09).
The full revision is also recorded in `Cargo.lock`; builds use `--locked` and
`deny.toml` permits only the named Git source with revision pins.

The fork starts at published 1.2.0's recorded source commit
[`43b3fe60db487758c195d6947f1c37e5246cedf2`](https://github.com/loco-rs/loco/commit/43b3fe60db487758c195d6947f1c37e5246cedf2),
from the release archive's `.cargo_vcs_info.json`. That archive's SHA-256 is
`5386baf405736a2620001f4561b00fa4874dae99eac3ccf01713fabc9ef13af4`.
The [fork diff](https://github.com/damacus/loco/compare/43b3fe60db487758c195d6947f1c37e5246cedf2...5224bc58f9292a2f4713ee333bded785e5266f09)
contains the scheduler fix, its tests and exact `shlex` dependency. Its `loco-gen`
dependency remains the published 1.2.0 package. Other locked package records are
unchanged. This avoids taking the unrelated framework changes in the current
upstream development branch; [upstream PR #1887](https://github.com/loco-rs/loco/pull/1887)
proposes the corresponding fix there.

Configured native jobs parse grouping and escaping with `shlex`, then execute
the application directly. Shell operators and expansions remain literal arguments.
Empty task names, malformed quotes and NUL bytes fail before spawning. Explicit
`shell: true` jobs still need a platform shell, which the scratch image lacks.
The released `JobDescription` fields and public shell-based `run()` API are retained;
direct execution and argument-safe failure logs apply to the scheduler-owned path.
Cron timing, filters, environment inheritance, output modes and shutdown remain
owned by the released framework.

The fork's scheduler tests cover exact native arguments, executable paths with
spaces, separate startup and periodic callbacks, output modes, nonzero exits,
missing executables and failure-log privacy. Keep those checks and the application's
process/image acceptance when updating the pin. Replace the fork with a published
release containing the correction when available; do not remove the regression
coverage with the temporary dependency.
Native `run` values support quoted arguments but do not expand shell syntax.
`shell: true` explicitly requests a platform shell and cannot run in this scratch
image. Keep production schedules as native tasks.

The **Loco Linux images** workflow builds and checks amd64 and arm64 on native
runners. Every push to **main** publishes a multi-platform
`ghcr.io/damacus/med-tracker-loco:sha-<full commit>` image and updates `beta`
after the native image checks, scheduler acceptance and image-transfer checks pass.
Dispatching **from main** with **publish** enabled also publishes; other
dispatches only check the images. Publication verifies checksums, image IDs and architectures of the
saved tested images, pushes those images, and joins their registry digests into
the multi-platform manifest. The publication job has no source checkout, build
or application execution. Pull requests and dispatches from other branches have
no package write permission and cannot publish. Transferred image archives
expire after one day.
The existing Rails image publisher remains separate. Publishing an image does
not deploy it. Both tags refer to the same manifest; the run summary records its
immutable digest and exact source commit. Pin that digest in GitOps. A build
after merging a pipeline change uses that subsequent main commit, rather than
the main commit observed before the change.

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
