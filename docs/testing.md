# Testing

Root `task ci` runs formatting, Clippy linting, database-free unit tests,
PostgreSQL 18 and RustFS integration tests, and Rust Playwright browser
journeys against the Loco application.

## Loco test stages

| Command                                                | Coverage                                                               |
|--------------------------------------------------------|------------------------------------------------------------------------|
| `task test:unit`                                       | Rust library and doc tests, plus JavaScript unit tests; no database    |
| `task test:integration`                                | API, access control, persistence, identity and queue integration tests |
| `task test:integration TARGET=care_api FILTER=fixture` | Focused Rust integration test binary and filter                        |
| `task test:browser`                                    | Browser journeys (`cargo nextest run --test browser`)                         |
| `task test:browser FILTER=barcode_miss`                | Run one browser journey by name substring                              |

Local integration runs provision disposable PostgreSQL 18 and RustFS containers
through `compose.yaml`; browser runs provision PostgreSQL only. Both ignore an
ambient `DATABASE_URL`. The
browser lane is a single job: Playwright is driven by the `playwright-rs` crate,
which assembles its driver in the `playwright-rust` cache directory and
Chromium in the usual `ms-playwright` cache. The driver supplies its own Node
runtime; `frontend:prepare` still uses Node and npm to build the application
CSS and scanner assets before the journeys run.
Journeys are marked `#[ignore]` so the shared integration lanes skip them;
`task test:browser` selects them with nextest `--run-ignored only`. Both
integration and browser tasks fail when a filter selects no tests.

The Rust browser lane is deliberately smaller than the retired JavaScript suite;
it does not claim test-for-test parity. Each journey chooses its viewport rather
than inheriting an automatic desktop/mobile matrix. Retained journeys now include
native passkeys and account forms, OAuth consent, care/scanner flows,
administration and location-deletion protection. CI uploads captures under
`docs/screenshots/browser-journeys/` as the `browser-journey-screenshots` artifact.

At [main `957e57390`](https://github.com/damacus/med-tracker/actions/runs/39883785308)
on 23 October 2026, CI executed 34 browser journeys successfully, with one
unignored harness unit test excluded by `--run-ignored only`. The same run passed
153 Rust library tests, 19 JavaScript unit tests and 1,169 tests in the integration lane
(including the 153 library tests; 38 additional tests skipped), plus the separately invoked template-concurrency guard. Counts
describe that run, not universal page, device or live-provider coverage.

The remaining concrete gaps have existing owners:
[typed browser fixtures](https://github.com/damacus/med-tracker/issues/2809),
[passkey cancellation/reuse browser paths](https://github.com/damacus/med-tracker/issues/2437),
[live provider/passkey acceptance](https://github.com/damacus/med-tracker/issues/1889)
and [accessibility](https://github.com/damacus/med-tracker/issues/2709).
Do not recreate the deleted suite or count API/render assertions as browser proof.

Running `cargo test --tests` directly requires an owned `DATABASE_URL`; the
empty `medtracker_reference` base database is created lazily by
`tests/support/template.rs` when absent. `task test:integration` provisions
the owned instance. The ignored `template_concurrency` guardrail runs automatically
on an unfiltered `task test:integration`.

Integration fixtures clone a migrated and seeded template for each fixture
family into private databases. Short-lived OAuth fixture credentials are created
for each clone so they cannot expire while waiting in a shared template.
Each browser journey clones the seeded `browser_template` into a private
database with a restricted runtime role, spawns the compiled `med-tracker`
server on a dynamic port, and drops the database, role and server process on
completion — including failure paths. Storage runtimes get private buckets.

## Browser journeys

Browser journeys live in `tests/browser/journeys.rs` as ordinary
`#[tokio::test]`s marked `#[ignore]` so the shared `--tests` lanes skip them.
Each journey supplies an explicit `Viewport` to `harness::journey(...)`, which clones the seeded
`browser_template` into a private database, spawns the compiled
`med-tracker` server, and drops everything on exit.

- Choose `task test:integration` for HTTP-level behaviour and
  `task test:browser` for a real browser driving the UI.
- To add a retained journey, write a test in `journeys.rs`, sign in with
  `harness::sign_in(&journey)`, drive pages with semantic locators
  (`get_by_role`/`get_by_label`) and `expect` assertions, and mark it
  `#[ignore]` like the others.
- Fixture data comes from SeaORM entity seeds in
  `tests/support/fixtures.rs`; add a seeding helper there rather than raw
  SQL. Template-level rows go through `harness`'s `prepare` callback;
  per-journey rows (for example `seed_barcode_metadata`) run inside the
  journey itself.
- `playwright-rs` assembles its driver at build time and
  `install_browsers` fetches Chromium on first run; both land in the user
  cache and need no separate Node install for Playwright. Application assets
  still use Node and npm through `frontend:prepare`.
- On failure the harness prints the tail of the server log; Playwright
  traces can be enabled per journey when more detail is needed.

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
