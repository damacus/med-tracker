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

## Local Rust verification and cache

Keep separate implementation worktrees, but serialise expensive Rust verification.
On this Mac, use the installed `medtracker-verify` runner from the source checkout:

```fish
rtk proxy medtracker-verify run --source "$PWD" -- ci SHARD=1/1
rtk proxy medtracker-verify run --source "$PWD" -- slice:test TARGET=care_api FILTER=example
rtk proxy medtracker-verify run --source "$PWD" --lane coverage -- test:coverage
```

The runner invokes Task in a stable copy of the source, holds one verification
lock, and separates normal and coverage outputs. Do not bypass an occupied lock
with a raw compiler or another target directory. Keep toolchain and build flags
consistent; do not create a new cache per agent or worktree. If the runner is
unavailable on another machine, use the repository Task commands with one build
owner; do not copy this Mac's absolute paths into repository configuration.

The machine-local cache uses one external root, a 20 GB sccache limit and a
120 GiB soft budget checked between runs. Use `rtk proxy medtracker-verify status`
to inspect it, `rtk proxy medtracker-verify prune` to preview cleanup, and append
`--apply` to prune under the verification lock. Never delete caches used by active
builds or introduce a conflicting `.cargo/config.toml` override. Raw Task commands
do not acquire this runner's lock automatically.

Results belong to the recorded source snapshot, not subsequent edits. Inspect
the lane's `source.json` and `last-run.json` under the external cache's
`verification/` directory. Screenshots and generated evidence are written in the
verification checkout; review and copy required artifacts back to the source
checkout before publication. Preserve the running review website. The runner
does not replace required checks or establish hosted CI acceptance.

## Application and delivery rules

Runtime settings come through `ctx.config.settings`, never `std::env` in `src/`.
Declare variables via `get_env` in `config/*.yaml`; preserve absent/null fallback
to stored settings. Use full `App::boot` Loco TestServer acceptance with restricted
runtime credentials, template-cloned PostgreSQL and ConnectInfo where needed.
Retain socket-specific coverage, real RLS, email stub/outbox and audit assertions.
Never enable global truncation or use administrator credentials at runtime.

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
