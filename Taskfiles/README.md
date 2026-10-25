# Task runner guide

Root Loco commands own the active Rust application. Rails tasks remain available
under `rails:*` for read-only reference work. Obsolete `rust:*` and `api:*`
namespaces and their migration Taskfiles have been removed. Recorded commands in
historical reports describe the tools that existed when those reports were written.

## Local Docker environments

Docker Compose v2.24.4 or later, Task, Python 3, curl and Docker are required.
All application ports bind to loopback and Docker assigns the host ports.
Run `task dev:ps`, `task test:ps` or `task prod:ps` to find them.
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

Development and test use the same pinned Rust, Node and Task toolchain.
Sources bind into the container. Build and download caches use named volumes,
avoiding macOS bind-mount I/O for compiler artifacts. Incremental compilation is enabled.
The application rebuilds frontend assets and reloads Rust, templates, CSS and
configuration through cargo-watch. Generated CSS and vendor JS are excluded from
the watcher to prevent restart loops.
Build development once, then start it. Rebuild the image after toolchain changes.

Each checkout has a project name derived from its absolute path, with a separate
suffix for dev, test and prod. Each environment owns its processes, ports, database
and storage. Dev and test share a target volume within the same checkout and native
Docker Linux architecture. Cargo fingerprints handle compiler and flag changes.
Different live worktrees retain separate targets: a shared final
`target/debug/med-tracker` could make a scheduler using `current_exe`, or a test
runner, execute another branch's program.

Cargo registry/Git and npm downloads are shared across worktrees. Mutable
`node_modules` stays within one checkout, keyed by package-lock, Node toolchain
and Linux architecture, and shared by dev/test. Standard Linux `flock` serialises
complete frontend prepare/install/build commands and watch-triggered preparation.
Its checkout lock lives in the npm download volume, outside `node_modules`, so
`npm ci` cannot remove the lock. Browser downloads share a cache
keyed by Linux architecture and the development image recipe. These caches are
explicit external volumes, so even Compose `down --volumes` leaves them intact.
Release BuildKit caches remain shared per architecture in the builder backend;
they are separate from runtime named volumes.

Existing environment-specific caches are retained; nothing is copied or deleted
automatically. To reuse this checkout's old warm dev target, replace `<dev-project>`
with the checkout's dev project name and pass its exact existing target volume:

```fish
env MT_TARGET_CACHE='<dev-project>_target' task test:unit TARGET=med-tracker FILTER=dependent_input_form
```

Use the same override for dev/test operations while reusing that volume. Only
this checkout's deterministic target name or its legacy dev/test target names are
accepted; data volumes and other worktrees' targets are rejected. The older test
target remains available until its owner explicitly chooses to clean it up.
Sharing removes unnecessary cache separation; no startup speedup is claimed.

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
