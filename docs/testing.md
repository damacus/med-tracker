# Testing

MedTracker uses RSpec, Capybara, and Playwright. Application tests run against
PostgreSQL 18.

## Prepare the test environment

Run the preflight before coding work:

```shell
task test:preflight
```

If it reports a missing image, build the test image before retrying. Use
`task test:rebuild` only when you need a destructive database reset.

## Run tests

Always run tests through `task`:

```shell
task test
```

Run a targeted spec path:

```shell
task test TEST_FILE=spec/models/user_spec.rb
```

The full suite includes browser and non-browser examples. A focused command can
target a file, directory, or line accepted by RSpec.

## Test environments

- Docker test environment: `task test` and `task test:*` tasks
- Local CI-like environment: `task local:*` tasks

Useful local commands:

```shell
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
2. Make the smallest change that passes.
3. Refactor while keeping tests green.

## Browser coverage

Browser examples use the `browser` tag and usually live under `spec/system/` or
`spec/features/`. Run all browser examples with `task playwright`, or pass a
specific file:

```shell
task playwright TEST_FILE=spec/system/dashboard_spec.rb
```

CI runs non-browser examples separately from two browser-test shards. Failure
screenshots and HTML are uploaded as CI artefacts.

## Linting

Run RuboCop through Taskfile:

```shell
task rubocop
task rubocop AUTOCORRECT=true
```

## Coverage

CI enables SimpleCov for the non-browser suite. The build requires at least 90%
line coverage and 75% branch coverage. The API group also requires 90% branch
coverage. These limits are defined in `.simplecov` and must not be lowered
without a recorded reason.

Focused local runs do not enforce the coverage gate unless `COVERAGE=true`.

## Mutation testing

Mutant checks whether selected specs detect changes to application code. Start
the test services, then choose a subject:

```shell
task test:up
task mutation SUBJECT=MedicationFriendlyName
task mutation SUBJECT='GlobalSearch::ResultBuilder*'
```

Use `task mutation:since` to check subjects changed since `origin/main`. Pass
`REF=HEAD~3` to compare with another Git reference. The CI mutation job is
advisory while its signal is evaluated.

Mutant uses its open-source mode. It does not need a licence token. See
`config/mutant.yml` for the current exclusions and their reasons.

## Test data

- Fixtures live in `spec/fixtures/`.
- Development seeding loads fixture-style data through `db/seeds.rb`.
- Test fixture users use the password `password`.
