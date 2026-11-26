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

Loco's released 1.2.0 scheduler required a correction for direct native-task
execution. The owner approved a temporary exception to the registry-only dependency
rule and patched `loco-rs` to
[`damacus/loco` commit `5224bc58f9292a2f4713ee333bded785e5266f09`](https://github.com/damacus/loco/commit/5224bc58f9292a2f4713ee333bded785e5266f09).
The 1.3.0 release includes shell-free scheduling and is used from the registry.
The application checks native scheduler commands for NUL bytes before starting.

The fork starts at published 1.2.0's recorded source commit
[`43b3fe60db487758c195d6947f1c37e5246cedf2`](https://github.com/loco-rs/loco/commit/43b3fe60db487758c195d6947f1c37e5246cedf2),
from the release archive's `.cargo_vcs_info.json`. That archive's SHA-256 is
`5386baf405736a2620001f4561b00fa4874dae99eac3ccf01713fabc9ef13af4`.
The [fork diff](https://github.com/damacus/loco/compare/43b3fe60db487758c195d6947f1c37e5246cedf2...5224bc58f9292a2f4713ee333bded785e5266f09)
contains the scheduler fix, its tests and exact `shlex` dependency. Its `loco-gen`
dependency was the published 1.2.0 package. [Upstream PR #1887](https://github.com/loco-rs/loco/pull/1887)
proposed the corresponding fix there.

Configured native jobs parse grouping and escaping with `shlex`, then execute
the application directly. Shell operators and expansions remain literal arguments.
Empty task names, malformed quotes and NUL bytes fail before spawning. Explicit
`shell: true` jobs still need a platform shell, which the scratch image lacks.
Cron timing, filters, environment inheritance, output modes and shutdown remain
owned by the released framework.

The fork's scheduler tests cover exact native arguments, executable paths with
spaces, separate startup and periodic callbacks, output modes, nonzero exits,
missing executables and failure-log privacy. The application retains its own
process and image acceptance checks when updating the framework.
Native `run` values support quoted arguments but do not expand shell syntax.
`shell: true` explicitly requests a platform shell and cannot run in this scratch
image. Keep production schedules as native tasks.

The **Loco Linux images** workflow builds amd64 and arm64 once each on native
runners. Both architectures run production boot and upgrade acceptance. The
native scheduler test starts when the checked amd64 image is available. Image
transfer verification waits for both architectures and the scheduler test.
Publication uses the saved, tested images to assemble the multi-platform
manifest after verifying their checksums, image IDs and architectures.
Transferred image archives expire after one day.

Pull requests run the image checks when matching files change. On main, Release
Please calls the workflow once for each new release. The workflow verifies that
the release tag identifies the workflow commit and that its version matches the
root Cargo package. It publishes the same multi-platform digest under
`ghcr.io/damacus/med-tracker:sha-<full commit>` and the full release version, for
example `ghcr.io/damacus/med-tracker:0.6.10`. The workflow summary records the
immutable digest to pin in GitOps. Publishing the image and deploying it are
separate operations.

If a previous run published a verified digest for the same commit, publication
reuses that digest. Release and manual publication for the same commit run one
at a time.

## Manual image publication

Run the workflow on main with **beta** enabled to build, check and publish its
current commit and point the `beta` tag at that run's verified digest.

```fish
gh workflow run loco-images.yml --repo damacus/med-tracker --ref main -f beta=true
```

A new beta request cancels any superseded beta build. A request with both
**publish** and **beta** enabled is a beta request and follows the same
cancellation rules. The promotion job checks main again before changing the
tag, so a run built from an older commit leaves beta pointing at its previous
digest. The workflow summary records whether the tag was updated or skipped.

For a hotfix, run the workflow on main with **publish** enabled to publish its
SHA tag.

```fish
gh workflow run loco-images.yml --repo damacus/med-tracker --ref main -f publish=true
```

Dispatches with default inputs or from other branches run the checks only.

## Creating the 0.6 release candidate

Release-please uses the root Rust package.
The one-time `release-as: 0.6.0-rc.1` setting selects the first candidate and
`prerelease: true` marks the GitHub release as a prerelease. The manifest records
the last released version; do not advance it manually to request a release.

1. Merge the release configuration and workflow change through the normal review
   process. Release-please updates its pending release PR to `0.6.0-rc.1`, including
   the Cargo files, changelog and manifest.
2. Review and merge that generated release PR. Release-please creates
   `v0.6.0-rc.1`, then calls the checked Rust image publisher. Both architectures,
   the production boot checks and native scheduler acceptance must pass before
   publication can run.
3. Record the image digest from the successful publication summary. Use that
   digest for server, worker, scheduler and schema setup through the deployment's
   existing GitOps controller. Follow the [fresh rollout notes](loco-cutover-runbook.md)
   for the separate database, credentials and runtime identity.
4. Remove or advance the one-time `release-as` override before the next release.
   For another candidate, set the explicit version, such as `0.6.0-rc.2`, while
   retaining `prerelease: true`. For the stable release, select `0.6.0` and set
   `prerelease: false`. Do not reuse a candidate tag for changed source.

The image workflow builds and publishes images only. It does not provision a
database or accounts, export or import clinical records, or change traffic. A
0.6 installation uses a new empty database and fresh accounts. Use the supported
portable-data contract when import or export is needed; its Rails compatibility
limits remain documented in [#2702](https://github.com/damacus/med-tracker/issues/2702).
This issue does not define a cutover plan or gate image publication.

`task image:check` compares the image's pinned Rust compiler with the compiler
selected by the application's stable CI policy, its Node version with `mise.toml`
and the exact Node patch selected by app CI, and its Task version with app CI.
The local check compares Node majors; CI additionally compares the resolved patch.
The image uses Rust 1.99.1 and Node 26.12.1;
`Cargo.toml`'s `rust-version` remains the minimum supported compiler. A new stable
release makes the alignment check fail until the Rust image version and verified
digest are updated. Node 26 and Task 3.52.0 match the application's selections.
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
