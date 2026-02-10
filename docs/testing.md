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

Useful local commands:

```bash
task local:test
task local:test:browser
task local:test:all
task local:clean
```

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
