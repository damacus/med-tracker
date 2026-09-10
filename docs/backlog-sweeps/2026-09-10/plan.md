# Five-issue test reliability tranche

## Outcome

Close #2095, #2099, #2140, #2145, and #2212 in one pull request and one commit. The change must keep
the real browser interactions, refresh stale mounted Playwright dependencies, and avoid sleeps,
retries, weaker assertions, dosing changes, or authorization changes.

## Scope

Nightingale owns one continuous tranche. Expected writable paths are:

- `spec/system/admin_manages_users_spec.rb`
- `spec/system/dashboard_spec.rb`
- `spec/system/schedules/dosage_selection_spec.rb`
- `spec/system/historical_dose_recording_spec.rb`
- `spec/config/taskfiles_spec.rb`
- `Taskfiles/test.yml`
- `scripts/test_preflight.fish`
- `scripts/test_dependency_verification.js`
- `scripts/test_capybara_browser.rb`
- `spec/support/test_dependency_verification_harness.js`
- `scripts/ci/policy.json`
- this tranche's report and ledger

Application components are read-only unless a deterministic product defect is reproduced. Stop and return to Bucky
before changing dosing rules, authorization, shared RubyUI components, dependency versions, Docker images, or the
Compose volume layout.

## Route

Planning route: Bucky plus Scout on Luna high; `writer_owner_count: 0`; Luna-to-Terra fallback only if Luna is
unavailable. Execution route: Bucky, one Nightingale on Luna high, and one independent Hubble on Luna high;
`writer_owner_count: 1`.

## Verification

Use Red-Green-Refactor. Record the failing evidence before each fix. Run:

```fish
task test TEST_FILE=spec/config/taskfiles_spec.rb
task test TEST_FILE=spec/system/admin_manages_users_spec.rb
task test TEST_FILE=spec/system/dashboard_spec.rb
task test TEST_FILE=spec/system/schedules/dosage_selection_spec.rb
task test TEST_FILE=spec/system/historical_dose_recording_spec.rb
task rubocop
task test
task docs:build
git diff --check
```

Repeat each affected browser example enough times to exercise controller readiness. Verify the Playwright preflight
against a stale test `node_modules` volume without replacing the browser revision with a compatibility link.

## Records and acceptance

- Brief: `docs/backlog-sweeps/2026-09-10/brief.md`
- Writer report: `docs/backlog-sweeps/2026-09-10/report.md`
- Independent review: `docs/backlog-sweeps/2026-09-10/review.md`
- Progress ledger: `docs/backlog-sweeps/2026-09-10/ledger.md`

Bucky accepts the tranche only after Hubble reports no Critical or Important finding and all applicable checks pass.
Bucky alone commits, pushes, opens the pull request, and verifies remote issue-closing references.
