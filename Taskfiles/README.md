# Task runner guide

Root Loco commands own the active Rust application. Rails tasks remain available
under `rails:*` for read-only reference work. Obsolete `rust:*` and `api:*`
namespaces and their migration Taskfiles have been removed. Recorded commands in
historical reports describe the tools that existed when those reports were written.

## Local Docker environments

Docker Compose v2.24.4 or later, Task, Python 3, curl and Docker are required.
All application ports bind to loopback and Docker assigns the host ports.
Portless provides stable HTTPS URLs named from the checkout/environment project,
for example `https://mt-<checkout-hash>-dev.localhost`. Run `mise install npm:portless`
for the pinned host CLI, then `portless doctor`. Start/trust the shared HTTPS proxy
explicitly if doctor requests it; these setup commands can require administrator
access. Lifecycle tasks never install services, trust certificates or start/stop
the shared proxy. `up` checks the trusted proxy before building containers.

After direct HTTP health succeeds, the task registers a static Docker alias and
checks health through its named HTTPS origin. Standard curl retries are bounded
to allow the proxy to notice the newly written alias before reporting readiness. App/scheduler configuration uses
that public origin, with secure cookies. `MEDTRACKER_PUBLIC_ORIGIN` is an optional
validated settings value separate from Loco server.host and the internal listen
port. Authentication origins, OAuth discovery/capabilities and invitation URLs
share this setting. Missing/null/empty preserves the existing server URL fallback;
configured production origins require HTTPS, and credentials/paths/query/fragments
are rejected. Loopback ports remain available in
`task dev:ps`, `task test:ps` and `task prod:ps`. Set `PORTLESS=0` explicitly to
bypass host proxy integration. Dev/test reuse their current owned port or ask
the OS for a free loopback port, then pass it as both the published port and
trusted public origin before starting the app. The short close-to-Docker-bind
race fails visibly on a conflict; there are no retries or port state files.
Normal Portless mode keeps Docker-assigned ports. Local production keeps secure cookies, so HTTP
bypass is only a health preview. If the shared HTTPS proxy uses a nondefault
port, set `PORTLESS_PORT` to match it. HTTPS and exclusively the .localhost suffix are required: Portless alias
commands affect every configured suffix, so doctor output is checked before
any alias mutation to avoid touching another suffix.

Aliases use the existing Compose app binding as ownership evidence. A conflicting
alias fails before startup; after recreation an alias may move from the observed
old app port to the new healthy port. Down/reset remove only an alias matching
the current owned app. If the stack has already gone, inspect the printed manual
cleanup command rather than guessing ownership. No force/prune or shared-proxy
stop is used, and no ownership files are created. Portless 0.15.11 has no JSON
list command, so the pinned CLI static-alias listing supplies the port check.

Alias commands disable automatic hosts-file synchronisation and LAN mode. Chrome
and Firefox resolve .localhost names directly; Safari may require an explicit
user-run `portless hosts sync`. The task verifies TLS routing using curl --resolve,
which does not alter system DNS or trust.
`task dev` is the same as `task dev:up`.

| Operation | Development | Test | Local production image |
| --- | --- | --- | --- |
| Build | `task dev:build` | `task test:build` | `task prod:build` |
| Start | `task dev:up` | `task test:up` | `task prod:up` |
| Stop | `task dev:down` | `task test:down` | `task prod:down` |
| Restart | `task dev:restart` | `task test:restart` | `task prod:restart` |
| Logs | `task dev:logs` | `task test:logs` | `task prod:logs` |
| Status | `task dev:ps` | `task test:ps` | `task prod:ps` |
| Synthetic seed | `task dev:seed` | `task test:seed` | Unavailable |

Development and test use the same pinned Rust, Node and Task toolchain. Node
packages belong to the image: one dependency stage runs `npm ci` using a BuildKit
npm download cache. Test and production reuse the same built frontend stage.
The frontend scanner runs with assets as its working root, excluding Cargo targets
and installed runtime files from discovery. Development mounts editable source
directories individually, keeping image-owned
`node_modules` visible. One Tailwind watcher writes development CSS; ordinary CSS
and JS edits neither compile Rust nor restart the backend. Tera edits update
Tailwind output and restart the web process using the existing binary.

One cargo-watch builder handles backend edits, avoiding duplicate web/scheduler
compilation. It installs a successful development executable into a dev-owned
runtime volume; consumers watch that final executable, not intermediate target
writes. Cargo fingerprints reuse unchanged compilation; startup still runs the standard installation step.
Without Git metadata, build.rs tracks only itself: generated CSS cannot invalidate
a metadata lookup that can only return unknown. Native Git checkouts retain their
existing metadata behaviour; Cargo still tracks Rust and embedded asset inputs. Embedded assets are
exceptions to frontend-only reload: the push-worker JS, icon, security password
list, catalogue, report locales/terminology and the two Rails reference YAML files
are Rust compilation inputs and are watched by the builder.

Every test invocation and `test:up` builds an immutable source snapshot, including
uncommitted source, tests, scripts and configuration allowed by `.dockerignore`.
Unchanged image layers are cached. Secrets, Git metadata and local build outputs
remain excluded. Tests have no source bind mounts, frontend writers or watchers. Their small container
Taskfile shares the canonical database tasks with the host root, without parsing
host-only Rails/mobile/worktree helpers.
Use `test:up` to refresh a running test stack; `test:restart` restarts its existing
snapshot. Rebuild/start development after changing Node dependencies or toolchains.

Each checkout has a project name derived from its absolute path, with separate
dev/test/prod processes, networks, ports, database and storage. Dev and test share
an external target volume within a checkout and native Docker Linux architecture,
with matching Cargo profile settings and incremental compilation. Cargo
fingerprints handle compiler/flag invalidation. Different live worktrees retain
separate targets: sharing final executables can make `current_exe` scheduler jobs
or runners execute another branch. The installed dev runtime also separates its
executable from a test snapshot using the shared compilation target.

Cargo registry/Git downloads are global external named volumes. Browser downloads
share a Linux architecture/Ubuntu 26 cache; the browser library selects its own
versioned browser directory. Named volumes avoid macOS bind-mount I/O for these
caches. External caches survive `down --volumes` and explicit data reset. npm and
release compilation caches stay in BuildKit, separate from runtime volumes.
Existing old cache volumes are retained without automatic copying or deletion.
Local verification used one temporary checkout on macOS/ARM64 Docker. These are
single observations, not a throughput benchmark or general startup claim.

| Observation | Result | Samples | Scope |
| --- | --- | --- | --- |
| Cold dev Cargo install | 4m43s | 1 | Fresh target, shared downloads |
| Rust main.rs edit/install | 20.84s | 1 | Existing dependency artifacts |
| Unchanged warm install | 2.08s | 1 | No compilation; still installs executable |
| Final public-origin candidate install | 1m24s | 1 | Reused target after focused request tests |
| Tailwind watcher before/after narrowed discovery | 1.403 GiB / 145.5 MiB | 1 each | Same temporary stack; Docker memory observation |

The cold/main.rs/warm observations preceded the public-origin correction; the
final-candidate row records its subsequent installation. The earlier concurrent
compiler run was killed by SIGKILL and is excluded from
timings. Cold/edit/warm rows describe different workloads and are not a
before/after speedup comparison.

Inherited Compose project/file/profile settings are ignored.
No arbitrary database URL or existing Compose project can be selected. Data volumes
without this workflow’s ownership label are rejected instead of adopted.

`down` preserves data and caches. `dev:reset` and `test:reset` require
`CONFIRM=<exact-project-name>`; the refusal message prints the required name.
Reset removes only that environment’s database and fixture-storage volumes.
Compiler and dependency caches survive. There is no production reset or seed task.
Seeding invokes the existing application guard: only an empty development or test
database can receive synthetic fixtures. No populated database is silently reset.

Startup runs migrations with the migration identity, then creates a restricted
runtime login belonging to `med_tracker_app`. Runtime is not a superuser and has
no database creation, role creation or RLS bypass privileges. Local passwords are
fixed synthetic credentials. Operational role provisioning uses createuser and a
single fixed ALTER ROLE statement because SeaORM cannot administer PostgreSQL roles.
Application queries and migrations retain their existing SeaORM boundaries.

`prod:*` runs only a local release-image check. The existing release build uses
`cargo --release` and the shell-free, non-root `FROM scratch` runtime, with no
source mounts or development toolchain. The local stack starts server, worker
and scheduler; it does not deploy anywhere. Production keeps secure cookies enabled.
The printed HTTP loopback URL supports health and page preview; authenticated
production browser use needs a local HTTPS endpoint. The task does not weaken
that application guard. Set `LOCO_IMAGE=<existing-local-tag>` to exercise an
existing release image, or use `prod:build` to build the current release image.
`task image:test` and `task test:scheduler:image` retain deeper production-image
and scheduled-write proof.

## Focused tests and CI

```fish
rtk task test:unit TARGET=med-tracker FILTER=domain_name
rtk task test:integration TARGET=care_api FILTER=fixture
rtk task test:browser FILTER=login
rtk task test:tooling
```

Unit TARGET selects a workspace package; integration/browser TARGET selects a Rust
test binary. Filters are passed as one argument and zero matches fail. Integration
and browser runners use isolated test services; unit tests start neither database
nor storage. The integration/browser harness creates its own restricted
runtime roles and template-cloned databases. `test:up` keeps an accessible test
application and services running. Stop them explicitly with `test:down`.

Hosted CI retains its host toolchain path when `CI=true` or when the existing
owned-database harness sets `MEDTRACKER_OWNED_DATABASE=1`. The host acceptance
path continues to provision and tear down disposable services when no owned
database is supplied. Explicit `host:test:unit`, `host:test:integration` and
`host:test:browser` are available for that path. `task ci` remains the application
gate; tooling-only changes use `task test:tooling` and relevant image checks.

## Other owners

Frontend assets use `frontend:*`; CLI/MCP use `client-tools:*`; generated clients
use `api-clients:*`; Android uses `mobile:android:*`; documentation uses `docs:*`.
`task -l` lists the current public surface. Do not use the retained old Rust source
as an active application or migration runner.
