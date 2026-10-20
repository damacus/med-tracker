# Testing

Root `task ci` runs formatting, Clippy linting, database-free unit tests,
PostgreSQL 18 and RustFS integration tests, and desktop/mobile Playwright
browser tests against the Loco application.

## Loco test stages

| Command                                                     | Coverage                                                               |
|-------------------------------------------------------------|------------------------------------------------------------------------|
| `task test:unit`                                            | Rust library and doc tests, plus JavaScript unit tests; no database    |
| `task test:integration`                                     | API, access control, persistence, identity and queue integration tests |
| `task test:integration TARGET=care_api FILTER=fixture`      | Focused Rust integration test binary and filter                        |
| `task test:browser`                                         | Desktop and mobile browser journeys                                    |
| `task test:browser GROUP=runtime-isolation PROJECT=desktop` | One browser file and viewport                                          |

Local integration and browser runs provision disposable PostgreSQL 18 and RustFS
containers through `compose.yaml` and ignore an ambient `DATABASE_URL`. In CI,
Playwright browser suites use native sharding (`SHARD=1/12` passes `--shard=1/12`
to Playwright).

Running `cargo test --tests` directly requires an owned `DATABASE_URL` plus a
provisioned `medtracker_reference` template database; `task test:integration`
provides both. The ignored `template_concurrency` guardrail runs automatically
on an unfiltered `task test:integration`.

Integration fixtures clone a migrated and seeded template for each fixture
family into private databases. Short-lived OAuth fixture credentials are created
for each clone so they cannot expire while waiting in a shared template.
The browser runner owns one PostgreSQL service and prepares one template for all
its workers. Each runtime gets a private database and role; storage runtimes get
private buckets. Between tests, workers restart the compiled application and
restore their data snapshot to clear both in-memory authentication limits and
database state.

## Loco database setup

Start with an empty PostgreSQL 18 database. The setup connection must be able
to create the required non-login roles and standard `citext`, `pg_trgm` and
`pgcrypto` extensions, assign ownership and assume `med_tracker_owner`.
Keep these credentials separate from the restricted application login.

| Command                                            | Result                                                         |
|----------------------------------------------------|----------------------------------------------------------------|
| `task db:migrate MIGRATION_DATABASE_URL=...`       | Complete schema, including functions, triggers, RLS and grants |
| `task db:prepare SEED_DATABASE_URL=...`            | Migrations followed by development fixtures                    |
| `task db:seed SEED_DATABASE_URL=... LOCO_ENV=test` | Migrations followed by test fixtures                           |

Loco owns schema installation through versioned SeaORM migrations. Ordinary
tables, indexes and foreign keys use builders; PostgreSQL-specific definitions
are bundled with their migration. The seed hook invokes the migrator once before
loading data.

Repeated migration is a no-op. Synthetic seeding refuses production environments
and populated application databases. Reset and down commands remain disabled;
create another empty database for a fresh start.

## TDD workflow

MedTracker follows Red-Green-Refactor:

1. Write a failing test first.
2. Make the smallest change that passes.
3. Refactor while keeping tests green.

For a manual screen-reader and keyboard pass over browser journeys, use the
[manual accessibility smoke-test checklist](accessibility-smoke-test.md).

## Reference Rails suite

The read-only Rails reference under `rails/` retains its own `task rails:*`
commands for reference inspection when needed:

```shell
task rails:test
task rails:rubocop
```

Test fixture users use the password `password`.
