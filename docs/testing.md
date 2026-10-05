# Testing

Root `task ci` verifies Loco formatting, lint, compilation and actual HTTP
health/Tera behaviour against a dedicated PostgreSQL 18 database. Set
`DATABASE_URL` to the disposable verification database. `task inventory:check`
and `task preservation:check` protect migration inputs and relocation records.
The Rails suite below runs through `task rails:*` and remains the reference for
product behaviour until each complete Loco journey passes its migration gates.

MedTracker uses RSpec, Capybara, and Playwright. Application tests run against
PostgreSQL 18.

## Prepare the test environment

Run the preflight before coding work:

```shell
task rails:test:preflight
```

If it reports a missing image, build the test image before retrying. Use
`task rails:test:rebuild` only when you need a destructive database reset.

## Run tests

Always run tests through `task`:

```shell
task rails:test
```

Run a targeted spec path:

```shell
task rails:test TEST_FILE=spec/models/user_spec.rb
```

The full suite includes browser and non-browser examples. A focused command can
target a file, directory, or line accepted by RSpec.

## Test environments

- Docker test environment: `task rails:test` and `task rails:test:*` tasks
- Local CI-like environment: `task rails:local:*` tasks

Useful local commands:

```shell
task rails:local:test
task rails:playwright
task rails:local:test:all
task rails:local:clean
```

`task rails:playwright` is the canonical local Playwright entrypoint. It runs the
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
`spec/features/`. Run all browser examples with `task rails:playwright`, or pass a
specific file:

```shell
task rails:playwright TEST_FILE=spec/system/dashboard_spec.rb
```

CI runs non-browser specs in two parallel file shards and browser examples in
two parallel example shards. Both use `scripts/ci/shard.mjs`. Historical timing
files are optional balancing hints: new specs need no timing entry, missing or
invalid durations use the mean of available timings, and without a timing file
items receive equal weights. Each non-browser file stays intact in one shard.

The coverage job combines both non-browser results and enforces the unchanged
coverage thresholds. It fails if either result is missing or duplicated. Failure
screenshots, HTML and coverage results are uploaded as CI artefacts.

RSpec tests application behaviour. CI script tests live in `scripts/ci/tests/`
and run with Node's built-in test runner through `task ci:test`, without Rails,
a database, Docker or an npm install. `task ci:check` runs workflow lint, script
syntax checks and these tests in the CI Workflow Check job.

`task ci:coverage:test` checks coverage collation with a standalone Ruby script
in the tools container, without loading Rails or RSpec. The coverage job runs
the same script before checking the application results.

## Linting

Run RuboCop through Taskfile:

```shell
task rails:rubocop
task rails:rubocop AUTOCORRECT=true
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
task rails:test:up
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
