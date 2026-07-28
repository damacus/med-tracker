# Task 7 Audition CI and baseline report

## Reviewed baseline

`task audition:baseline` completed with Audition 0.2.1 on Ruby 4.0.6 and wrote 26 findings to `.audition-baseline.json`.

The baseline groups match the application-owned inventory in `docs/audition-readiness.md`:

- 20 application findings: 17 mutable constants and 3 class-level state findings.
- 3 library mutable-constant findings.
- 3 configuration findings: 1 Pagy mutable-constant warning and 2 OpenTelemetry unsafe-call informational findings.

The 21 grouped baseline keys sum to 26 findings. The three script main-guard findings remain outside the reviewed production baseline.

## CI contract

The `audition` GitHub Actions job builds the pinned Audition scanner from the Docker `tools` stage. It runs the reviewed baseline-aware static scan as a blocking step.

Dependency and dynamic scans are named advisory steps with `continue-on-error: true`. The dynamic step receives a PostgreSQL 18 service through host networking, prepares the test database, and then runs the dynamic probe. Audition remains installed at `/opt/audition` outside the application bundle.

## Verification

- RED: `task test TEST_FILE=spec/config/audition_toolchain_spec.rb` failed with 5 examples and 1 failure because `jobs.audition` did not exist.
- GREEN: the same focused spec passed with 5 examples and 0 failures after the workflow was added.
- `task audition:ci` passed with `audition verdict: ractor-ready as far as audition can tell`.
- `task audition:static` intentionally exited nonzero without the baseline and reported the known inventory: 23 errors, 1 warning, and 2 informational findings.
- `task audition:dependencies` remained advisory and exited nonzero, reporting 50 of 273 locked gems as Ractor-ready.
- `task audition:dynamic` remained advisory and exited nonzero locally because the existing local tools service did not receive a database URL. The CI job supplies PostgreSQL 18 access and runs `db:prepare` before the probe, but its dynamic result remains advisory until observed in GitHub Actions.

No Audition fixes were applied, and Audition was not added to `Gemfile` or `Gemfile.lock`.
