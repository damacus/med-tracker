# Testing

MedTracker uses RSpec with Capybara/Playwright and runs tests in PostgreSQL-backed
test environments.

## Standard test command

Always run tests through `task`:

```bash
task test
```

Run a targeted spec path:

```bash
task test TEST_FILE=spec/models/user_spec.rb
```

## Test environments

- Dockerized test environment: `task test` and `task test:*` tasks
- Local CI-like environment: `task local:*` tasks

The test stack preserves the same connection boundary as a deployment:

- `migrate-test` connects as `medtracker_migration` and sets
  `med_tracker_owner` while applying migrations.
- `web-test` connects as `medtracker_runtime` and sets `med_tracker_app` for
  server behavior.
- The ephemeral `test-runner` is the only Rails service given the bootstrap credential.
  Rails fixture maintenance needs PostgreSQL system-trigger and forced-RLS access
  that the runtime and migration logins intentionally lack.

The fixture exception exists only in the test profile. Do not copy the
`test-runner` credential into `web-test`, development, production, web, job, or
migration services. CI likewise scopes bootstrap access to fixture-backed RSpec
and seed steps; migrations and server processes still use their isolated logins.

Useful local commands:

```bash
task local:test
task playwright
task local:test:all
task local:clean
```

`task playwright` is the canonical local Playwright entrypoint. It runs the
browser-backed system tests through the repo's Taskfile wrapper.

For a manual screen-reader and keyboard pass over those journeys, use the
[manual accessibility smoke-test checklist](accessibility-smoke-test.md).

## TDD workflow

MedTracker follows Red-Green-Refactor:

1. Write a failing test first.
2. Implement the minimal change to pass.
3. Refactor while keeping tests green.

## Browser/system coverage

System and feature tests live under `spec/system/` and `spec/features/`.
Browser flows use Playwright in CI and local browser-enabled runs.

## Linting

Run RuboCop through Taskfile:

```bash
task rubocop
task rubocop AUTOCORRECT=true
```

## Test data

- Fixtures live in `spec/fixtures/`
- Development seeding loads fixture-style data through `db/seeds.rb`
- Test fixture users use password: `password`
