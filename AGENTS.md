# MedTracker Rust agent guide

Read and apply these repository skills before Rust work:

- [.agents/skills/rust-loco-architecture/SKILL.md](.agents/skills/rust-loco-architecture/SKILL.md)
- [.agents/skills/rust-tooling/SKILL.md](.agents/skills/rust-tooling/SKILL.md)
- [.agents/skills/rust-drift-audit/SKILL.md](.agents/skills/rust-drift-audit/SKILL.md)

Read the global `start` skill before new work. Discover and call Serena initial
instructions before broad exploration; if unavailable, state that and use direct
repository tools. Use Context7 for current dependency/API details when available.

Loco 1.2 is the root application (`src/`, `migration/`, `config/`, `assets/`).
Views use Tera and DaisyUI; PostgreSQL is version 18. Rails under `rails/` is
read-only reference, with its own guide. Rust is fresh: no Rails data import,
credential migration, schema parity or reverse migration. Never reset or silently
adopt an existing populated database. No production deployment is authorised.

Read [the accepted decisions](docs/design.md#accepted-rust-direction). Architecture
acceptance is not proof of implemented behaviour. Preserve API/MCP contracts except
explicitly superseded access/location semantics; update OpenAPI, clients and ADRs
together for deliberate changes.

Use Fish syntax and prefix commands with RTK. Use `task` entry points; run
`task ci` for application changes. For docs run `task docs:build` and
`git diff --check`; tooling changes need their focused tests. Write and observe
a failing test before executable changes, then make it pass and refactor.
Do not change comments except for requested documentation/guardrails or lint fixes.
Do not introduce broad lint suppressions, including tests and generated entities.
Use `jq` for JSON inspection. Fixture passwords are `password`.

## No Python

Do not write or use Python for repository implementation, setup, build orchestration,
helpers, code generation, validation or tests. This includes temporary scripts and
one-off commands, not just committed files. Use native tool commands, Task/Compose,
Fish, or the existing Rust/JavaScript test facilities. Replace existing Python
helpers when changing the workflow they serve; do not extend them or introduce
another scripting framework to replace them.

## Local Rust verification

Run the repository Task entries directly from your checkout:

```fish
rtk task ci
rtk task test:integration TARGET=care_api FILTER=fixture
rtk task test:browser
```

Integration and browser tasks provision disposable PostgreSQL (and RustFS
where the suite needs it) through `compose.yaml`; they do not require a
pre-existing database. When `MEDTRACKER_OWNED_DATABASE=1` and `DATABASE_URL`
are already set, the same tasks reuse that instance instead of provisioning.

## Application and delivery rules

Runtime settings come through `ctx.config.settings`, never `std::env` in `src/`.
Declare variables via `get_env` in `config/*.yaml`; preserve absent/null fallback
to stored settings. Use full `App::boot` Loco TestServer acceptance with restricted
runtime credentials, template-cloned PostgreSQL and ConnectInfo where needed.
Retain socket-specific coverage, real RLS, email stub/outbox and audit assertions.
Never enable global truncation or use administrator credentials at runtime.

Use SeaORM entities, relations, expressions and query builders for database access,
including joins, filtering, pagination, aggregation and locking. Raw SQL strings,
custom SQL fragments and direct driver/backend-specific APIs (for example SQLx
PostgreSQL pools, rows or query adaptors, or hard-coded PostgreSQL query builders)
are prohibited when SeaORM can express the required behaviour. Do not bypass
SeaORM for convenience, familiarity or an assumed performance benefit. This rule
also applies to migrations, fixtures and tests.

An exception is allowed only when SeaORM cannot provide the required functionality.
Before using one, identify the exact unsupported operation and the SeaORM APIs
checked; document that limitation and the narrow exception in the code and PR.
Keep the exception isolated, parameterise all values, preserve tenant/RLS and
transaction boundaries, and cover the behaviour with focused acceptance tests.
Use the connection-provided backend when constructing statements; hard-code a
backend only when its specific functionality is the documented limitation.

Use published release versions for package dependencies, never Git SHA or unreleased
branch dependencies. Retain committed lockfiles and artifact integrity checksums.
The owner-approved temporary exception is the exact `damacus/loco` revision in
`Cargo.toml` for shell-free scheduler execution, based on the published 1.2.0 source.
Keep the full revision and lockfile pinned; return to a published release when the
scheduler correction is available. This exception does not cover other dependencies.

Use maintained protocol and crypto libraries. Search dependencies before custom
security code; document an exact limitation and interoperability/negative tests
if custom work is unavoidable. Better Auth is not proved faulty. Keep live
permission checks, RLS, replay protection, atomic stock deduction and mail outbox.

Verify visible UI through the browser at desktop/mobile sizes with screenshots
under `docs/screenshots/`. Keep keyboard access, labels, errors, focus, contrast
and dialog handling accessible. Never put queries in template loops.

Use Conventional Commits. Before every commit verify author, committer and signing
identity. If 1Password cannot sign, disable signing for that commit only. Preserve
external worktrees. Publication belongs to the delivery controller; do not merge,
deploy or act on live data. Report local, published, accepted and deployed states
separately. Lefthook recreation is delivery Task 4, not a Task 1 success claim.

Use Sol for implementation and routine review. Reserve Astra for security reviews.
Publish completed, independently verified slices as separate PRs against main;
do not hold finished guidance or tooling behind the application refactor.
