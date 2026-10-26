# Task runner guide

Task runs on the host and calls Docker Compose directly. Containers run native
Cargo, Node, Tailwind and Loco commands. There is no Python lifecycle wrapper or
container Task installation. Rails tasks remain read-only references under
`rails:*`; obsolete `rust:*` and `api:*` namespaces are removed.

## Commands

Host prerequisites: Docker Compose 2.24.4+, Task, Fish, curl and jq.
`task host:docker` checks them. Install the pinned host Portless CLI with
`mise install npm:portless`, then run `portless doctor`. Start/trust the shared
HTTPS proxy explicitly if requested; this may need administrator access.
Tasks never change system trust, start/stop that proxy or force aliases.

| Operation | Development | Test | Local release image |
| --- | --- | --- | --- |
| Build | `dev:build` | `test:build` | `prod:build` |
| Start | `dev:up` | `test:up` | `prod:up` |
| Stop | `dev:down` | `test:down` | `prod:down` |
| Restart | `dev:restart` | `test:restart` | `prod:restart` |
| Logs | `dev:logs` | `test:logs` | `prod:logs` |
| Status | `dev:ps` | `test:ps` | `prod:ps` |
| Synthetic seed | `dev:seed` | `test:seed` | Unavailable |

Prefix commands with `task`; `task dev` aliases `dev:up`.
`Taskfiles/compose.yml` holds shared native Compose operations; small environment
Taskfiles expose their commands. `Taskfiles/host.yml` holds host prerequisites and
native Rust checks. CI calls `host:test:*` explicitly; local `test:*` uses Docker.

The project name hashes the checkout's absolute path. Each environment has
separate processes, networks, loopback ports and database/storage volumes.
Compose files/project are explicit and inherited profiles are cleared.
Existing data volumes require the project's ownership label. Startup migrates
with the migration identity, then validates a restricted runtime login without
superuser, database/role creation or RLS bypass privileges. Fixed local role
administration is a narrow exception because SeaORM cannot manage PostgreSQL roles.

## Public URL

`host:portless` checks the existing trusted HTTPS proxy before building.
After direct HTTP health, a small Fish adapter registers the project's
`.localhost` alias and checks trusted HTTPS health with bounded curl retries.
The current Compose app port proves alias ownership; conflicts fail and down
removes only a matching alias. No ownership files are created. The proxy must
serve exclusively `.localhost`, since alias commands affect every configured
suffix. Hosts-file sync/LAN mode stay disabled. Set `PORTLESS_PORT` for an
existing proxy on a nondefault port.

For explicit proxy bypass, choose a free loopback port on first startup:

```fish
rtk env PORTLESS=0 task dev:up PORT=18150
rtk env PORTLESS=0 task dev:down
```

Subsequent up reuses the owned port. A conflicting port fails visibly.
The port is known before app start so the trusted public origin matches its URL.
Normal Portless mode keeps Docker-assigned ports. Production keeps secure cookies;
its HTTP bypass is health-only. The validated `MEDTRACKER_PUBLIC_ORIGIN` remains
separate from the internal listen address; forwarded request headers are not trusted.

## Builds and data

Dev/test share pinned Rust/Node tooling. npm dependencies belong to an image
stage with BuildKit download caching; test/prod share built frontend output.
Explicit dev source mounts keep image-owned `node_modules` visible.
One Tailwind watcher scans only assets. Ordinary CSS/JS edits do not compile or
restart Rust. Tera edits rebuild CSS and restart web with the existing binary.

One cargo-watch builder installs successful dev builds into a dev-owned runtime
volume. Web/scheduler watch that executable, avoiding duplicate compilation and
preventing an older test snapshot from replacing the live dev executable.
Embedded worker JS, icon, security/catalogue/report assets and two reference
YAML files remain Rust compilation inputs. Cargo fingerprints provide incremental
reuse. Source-only image builds track `build.rs` itself rather than unavailable
Git metadata; native checkouts retain their existing Git metadata behaviour.

Every test invocation and `test:up` builds an immutable current source snapshot,
including allowed uncommitted files. Unchanged layers are cached; secrets, Git
metadata and build outputs remain excluded. Tests have no source mounts, frontend
writers or watchers. `test:restart` retains its snapshot; `test:up` refreshes it.
Rebuild dev after dependency/toolchain changes.

Dev/test share an external target per checkout/native Docker architecture.
Different live worktrees keep separate targets to prevent executable collisions.
Registry/Git downloads are global external volumes. Browser downloads use an
architecture/Ubuntu cache with library-versioned directories. Named volumes avoid
macOS bind-mount cache I/O. BuildKit npm/release caches remain separate.
Older cache volumes are never automatically copied or deleted.

Down preserves data/caches. Dev/test reset requires
`CONFIRM=<exact-project-name>` and removes only that environment's database and
fixture-storage volumes. External compiler/download caches survive.
Seed retains the empty-database application guard; populated data is never reset
or adopted. Production has no public seed/reset commands and never deploys.
`LOCO_IMAGE=<existing-local-tag>` selects an existing release image for prod up.
The shell-free, non-root scratch runtime remains unchanged.

## Focused checks

```fish
rtk task test:unit TARGET=med-tracker FILTER=domain_name
rtk task test:integration TARGET=care_api FILTER=fixture
rtk task test:browser FILTER=login
rtk task test:tooling
rtk task host:fmt
rtk task host:lint
rtk task host:test:integration TARGET=care_api FILTER=fixture
```

Unit TARGET selects a package; integration/browser TARGET selects a test binary.
FILTER is one argument; zero matches fail. Units start no database/storage.
Integration/browser services belong to the test project; their harness uses
restricted roles/template databases. Unfiltered integration retains the ignored
cross-process template acceptance. `host:ci` (also `ci`) is the native application
gate; tooling changes use focused tests and image checks.

Nextest runs four tests concurrently by default. Override its native setting for
either Docker or host tests, for example:

```fish
env NEXTEST_TEST_THREADS=2 rtk task test:integration TARGET=care_api FILTER=fixture
```

The database-role mutation test still reserves every runner slot. Docker
compilation remains limited to two jobs. Test concurrency is configured separately.

`MT_TARGET_CACHE` can select an existing compatible Linux target volume for a
test run. It changes the compiler cache only; networks, databases and processes
retain their normal project isolation. Cargo supplies its own build-directory lock.

Previous macOS/ARM64 observations (one sample each): cold Cargo install 4m43s,
Rust edit/install 20.84s, unchanged install 2.08s. These preceded the public-origin
correction (subsequent install 1m24s). Different workloads are not a before/after
speedup comparison. This host-orchestration simplification claims no new compile
speedup.

Other owners: `frontend:*`, `client-tools:*`, `api-clients:*`,
`mobile:android:*`, `docs:*`. `task --list` shows the public commands.
