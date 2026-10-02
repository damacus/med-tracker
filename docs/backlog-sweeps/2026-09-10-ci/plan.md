# CI runtime and validation lanes plan

## Outcome

Resolve issues #2129, #1934, #1935, #1936, and #2084 in one CI and test-infrastructure tranche.
The tranche must keep all existing behaviour, coverage thresholds, Android variants, browser examples, and
Lighthouse score thresholds.

## Scope

Nightingale is the only writer. Expected writable paths are:

- `.github/workflows/ci.yml`
- `Dockerfile`, `Gemfile`, `Gemfile.lock`, `package.json`, and `package-lock.json` only for #2129
- `Taskfiles/test.yml` and directly related CI tasks
- `scripts/ci/` and narrowly related test helpers
- `spec/config/` and `spec/support/` contract tests
- `spec/requests/admin/audit_logs_rate_limiting_spec.rb` and measured fixture declarations for #1934
- `mobile/android/Taskfile.yml` and a minimal Android task contract test for #2084
- `docs/backlog-sweeps/2026-09-10-ci/`

Application code, database roles, authentication, authorisation, health data, deployment, score thresholds,
coverage thresholds, and release behaviour are out of scope. Do not add retries, sleeps, compatibility browser
aliases, skipped Android variants, or advisory success paths.

## Tasks

1. Align the Ruby Playwright client with the npm Playwright runtime and installed browser revision. Remove stale
   compatibility aliases. Prove a fresh image launches an actual Capybara browser example without a cache override.
2. Profile the current non-browser lane in its existing CI-equivalent environment. Reduce the remaining repeated
   rate-limit request work with test-only thresholds and remove only measured fixture overloading. Partition the
   non-browser examples into two measured file shards, preserve boundary behaviour, isolate coverage resultsets, and
   collate them before enforcing the unchanged coverage gates. Record before-and-after wall time. Do not redesign the
   runner environment.
3. Make Lighthouse depend only on change classification and required setup. Select it only for UI-relevant paths,
   keep uploads and thresholds, and make the aggregate gate fail for selected failed, cancelled, or missing results.
4. Build an authoritative browser-capable test list from RSpec metadata. Assign each browser item exactly once to two
   deterministic, duration-balanced shards. Keep failure screenshots per shard and record the timing evidence used.
5. Cap Gradle workers at one for the all-variant Android assembly task. Keep the 4 GiB heap, stacktraces, all six APK
   variants, tests, lint, release-security checks, Wear boundary checks, and pinned API verification.

## Assistance and ownership

Planning used Bucky, a read-only Scout on Luna high, and a bounded read-only Astra medium judgement for the
cross-cutting CI scope. `writer_owner_count: 0` during planning. Execution uses Bucky, one Nightingale on Luna high,
and one independent Hubble reviewer. `writer_owner_count: 1` during execution.

## Verification

Use Red-Green-Refactor for each contract. Run the smallest failing spec or harness first, then:

- the affected `spec/config/` and metadata contract specs through `task test TEST_FILE=...`;
- the focused audit-log rate-limit spec and CI-equivalent non-browser lane with timing and unchanged coverage gates;
- the two measured non-browser shards, raw SimpleCov artifact uploads, exact two-input collation, and coverage failure
  paths;
- syntax and repository policy checks for workflow and helper changes;
- `task test:build NO_CACHE=true`, `task test:preflight`, and a representative browser spec without a browser-path
  override;
- `task android:ci`, followed by repeated fresh-runner GitHub Android checks;
- `task rubocop` for changed Ruby files, `task docs:build`, `git diff --check`, and the full applicable suite.

The repository rule does not require Rails preflight or the Rails suite solely for CI, Android, documentation, or
non-production test tooling. Run the full Rails suite only if the final diff changes Rails code or the CI contracts
need it for authoritative verification.

## Escalation and acceptance

Stop and return to Bucky if Playwright alignment needs a wider dependency upgrade, duration data cannot support a
balanced shard claim, the non-browser lane still exceeds ten minutes after measured reductions, or one-worker Android
packaging still exhausts heap. Do not guess at more heap or runner memory.

Hubble must issue separate requirements and code-quality verdicts for each issue. Critical and Important findings
return to the same Nightingale. After the per-issue review is clean, Hubble performs one broad whole-tranche review.
Bucky alone accepts, commits, pushes, opens the pull request, verifies linked issues, and records remote CI evidence.
