# Contributing to MedTracker

## Quick setup

```fish
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
mise trust && mise install
task deps
task build
task db:prepare SEED_DATABASE_URL=postgres://...  # schema plus synthetic fixtures
task dev                       # start the Loco application
```

Use the address printed by the server.

Install Docker, Task, Node.js, npm, mise, and Git before running these commands.
The [technical quick start](docs/quick-start.md) explains database setup and the
development stack. Integration and browser tasks provision their own disposable
PostgreSQL through `compose.yaml`.

## Development workflow

```fish
task test:unit                 # database-free checks, doctests and Node test files
task test:integration          # API and database behaviour in Docker
task test:browser              # Rust Playwright journeys
task ci                        # fmt, lint and all test stages
task fmt                       # cargo fmt --check and related format gates
task lint                      # clippy -D warnings and dependency gates
task docs:serve                # serve docs locally
```

## Standards

### Testing

- Follow Red-Green-Refactor: write a failing test before executable changes,
  then make it pass and refactor
- **cargo-nextest** runs the suite (`--no-tests=fail`); `cargo test` is reserved
  for doctests
- Integration tests share one run-owned, pre-migrated PostgreSQL database with
  generated scenario IDs and tenant-scoped assertions
- Browser tests use **Playwright** via Rust test binaries; screenshot evidence
  lives under `docs/screenshots/`
- Test through public APIs only; tests document expected business behaviour
  rather than code details
- Fixture passwords are `password`

### Code style

`cargo fmt` and `cargo clippy --all-targets -- -D warnings` are enforced through
`task fmt` and `task lint`. Key rules:

- Guard clauses instead of nested `if/else`
- Strong typing; no `Any`-style escape hatches
- Small, single-responsibility functions
- Do not change comments unless the task requires it
- Use SeaORM entities, relations and query builders for database access; no raw
  SQL strings where SeaORM can express the behaviour

#### Naming

| Thing | Convention |
|---|---|
| Functions / variables / modules | `snake_case` |
| Types / traits / enum variants | `PascalCase` |
| Constants | `UPPER_SNAKE_CASE` |
| Files | `snake_case.rs` |
| Test targets | `tests/<area>.rs` |

### Architecture

- The product is a **Loco 1.2** application: domain logic lives in models and
  services; controllers stay thin
- Views are **Tera** templates with **DaisyUI**; never run queries in template
  loops
- Persistence uses **SeaORM** with PostgreSQL row-level security; runtime
  credentials are restricted
- Settings come from `ctx.config.settings`; declare environment variables via
  `get_env` in `config/*.yaml`
- The Rails application and the earlier Axum port are archived in git history;
  do not resurrect them or import their internals

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add prescription management feature
fix: resolve dose timing validation error
docs: update CONTRIBUTING with setup steps
refactor: extract dosage calculation to a model
test: add coverage for medication take restore
chore: bump Rust toolchain
```

- Each commit is a complete, logical unit of work with green checks before merge
- Avoid "initial commit" or "wip" messages

## Stack reference

| Layer | Technology |
|---|---|
| Language | Rust (stable) |
| Framework | Loco 1.2 (Axum) |
| Database | PostgreSQL 18, SeaORM, row-level security |
| Frontend | Tera, DaisyUI, TailwindCSS |
| Testing | cargo-nextest, Playwright, node:test |
| Authentication | Better Auth, passkeys, OIDC |
| Task runner | [Task](https://taskfile.dev) (`Taskfile.yml`) |
| CI | GitHub Actions (`.github/workflows/`) |
