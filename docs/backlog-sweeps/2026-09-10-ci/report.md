# CI runtime and validation lanes report

## Fix summaries

fix(ci): align the Ruby Playwright driver with its Chromium runtime

This pins the published Ruby Playwright client at 1.62.0 and npm Playwright at 1.63.0, the pairing that installs Chromium revision 1243 for the Ruby driver. The image uses that aligned runtime directly.

fix(ci): reduce repeated audit-log rate-limit requests

This keeps the boundary assertions while using a test-local limit of three and only the explicit account, people, and user fixtures.

fix(ci): make Lighthouse a selected required lane

This runs Lighthouse from change classification for UI-relevant paths and includes its result in the aggregate success gate without changing uploads or score thresholds.

fix(ci): balance browser examples from RSpec metadata

This builds the complete browser example list from RSpec metadata and assigns each item exactly once to two deterministic duration-balanced shards using measured timings.

fix(android): serialize the all-variant assembly workload

This passes `--max-workers=1` to the existing all-variant Gradle assembly while retaining the six APK variants and all existing CI checks.

## Implementation and changed files

- #2129: `Dockerfile`, `Gemfile`, `Gemfile.lock`, `package.json`, `package-lock.json`, `spec/config/ci_runtime_lanes_spec.rb`.
- #1934: `.github/workflows/ci.yml`, `.simplecov`, `scripts/ci/collate_simplecov.rb`, `scripts/ci/non_browser_shards.mjs`, `scripts/ci/non_browser_timings.json`, `scripts/ci/policy.json`, `spec/config/ci_runtime_lanes_spec.rb`, `spec/config/non_browser_ci_shards_spec.rb`, `spec/config/simple_cov_spec.rb`, `spec/requests/admin/audit_logs_rate_limiting_spec.rb`, `docs/testing.md`.
- #1935: `.github/workflows/ci.yml`, `scripts/ci/policy.json`, `spec/config/ci_runtime_lanes_spec.rb`.
- #1936: `.github/workflows/ci.yml`, `scripts/ci/browser_shards.mjs`, `scripts/ci/browser_timings.json`, `spec/config/ci_runtime_lanes_spec.rb`, `spec/config/test_lane_metadata_spec.rb`.
- #2084: `mobile/android/Taskfile.yml`, `spec/config/android_taskfile_spec.rb`.
- Report: `docs/backlog-sweeps/2026-09-10-ci/report.md`.

No heap size, Android variant, application behaviour, coverage threshold, Lighthouse threshold, issue, PR, or review file was changed.

## Red and Green evidence

### #2129

- Red: `task test:build NO_CACHE=true` with the attempted Ruby `playwright-ruby-client` 1.63.0 failed during Bundler resolution because that version is not published.
- Red: with npm Playwright 1.62.0, `task test TEST_FILE="spec/system/user_sessions_spec.rb[1:1]"` failed because the Ruby driver requested `/ms-playwright/chromium_headless_shell-1243` while npm 1.62.0 installed revision 1234.
- Green: `task test:build NO_CACHE=true` completed and installed Chromium and headless shell revision 1243 from npm Playwright 1.63.0.
- Green: `task test TEST_FILE="spec/system/user_sessions_spec.rb[1:1]"` ran 3 examples with 0 failures through Capybara Playwright.
- Green: `task test:preflight` refreshed mounted node_modules from `package-lock.json`, passed both the Capybara Playwright launch and Chromium launch checks, and passed 38 focused Taskfiles examples.

### #1934

- Remote trigger: GitHub job `103033241514` (`Run non-browser tests`) took 15m31s, exceeding the ten-minute follow-up threshold.
- Baseline: `task test TEST_FILE="spec --tag '~browser' --format progress"` ran 5986 examples with 0 failures and 1 pending in 10m01s.
- Green focused boundary: `task test TEST_FILE=spec/requests/admin/audit_logs_rate_limiting_spec.rb` ran 2 examples with 0 failures after the local threshold and fixture reduction.
- Comparable after: `task test TEST_FILE="spec --tag '~browser' --exclude-pattern 'spec/config/ci_runtime_lanes_spec.rb' --exclude-pattern 'spec/config/android_taskfile_spec.rb' --format progress"` ran 5992 examples with 0 failures and 1 pending in 7m15s. The two new contract files were excluded from this timing command; the remaining count reflects the current checkout.
- Follow-up measurement: `task test TEST_FILE="spec --tag '~browser' --format json --out tmp/non-browser-timings.json"` completed in 7m18s with 6003 examples, 2 contract failures, and 1 pending; the failures were the intentionally red shard/coverage contracts during measurement.
- Green measured manifest: `scripts/ci/non_browser_timings.json` contains 792 spec-file timings totalling 436.9670215530002s. The helper estimated shard 1 at 218.48350940399982s and shard 2 at 218.4835121490001s, with 396 files each and every file kept intact.
- Green exact-once check: concatenated shard output contained 792 lines and 792 unique file paths.
- Green final contracts: `env COVERAGE=true SIMPLECOV_SHARD=true task test TEST_FILE="spec/config/non_browser_ci_shards_spec.rb spec/config/simple_cov_spec.rb spec/config/ci_runtime_lanes_spec.rb"` ran 29 examples with 0 failures, including missing/invalid timing, shard index, failing-shard propagation, dry-run isolation, SimpleCov expected-command/non-empty/malformed/duplicate input validation, merge/shortfall cases, and aggregate coverage gate outcomes.
- Remote Red isolation: GitHub job `103057136889` showed the profile as Member because the browser spec relied on shared membership state; synthetic collation also overwrote the Rails coverage directory.
- Green isolation: `task test TEST_FILE="spec/config/non_browser_ci_shards_spec.rb spec/system/two_factor_soft_enforcement_spec.rb"` ran 12 examples with 0 failures after explicitly establishing the owner membership and routing every synthetic collator through a unique temporary `SIMPLECOV_COVERAGE_DIR`.

### #1935

- Red: the initial CI contract reported no independent Lighthouse selection, no Lighthouse workflow output/policy job, and an advisory Lighthouse dependency.
- Green: the final CI/config contract run passed 29 examples with 0 failures, including independent `needs: changes`, UI-only policy selection, executable UI/non-UI classification, Lighthouse aggregate-gate coverage for success, failure, cancelled, missing, and unselected results, and aggregate coverage failure/cancelled/missing outcomes.

### #1936

- Red: the initial CI contract found filename discovery and modulo file sharding.
- Green: `task test TEST_FILE="spec --tag browser --format json --out tmp/browser-timings.json"` ran 245 examples with 0 failures in 5m21s.
- Green: the committed measured timing map produced estimated shard loads of 990.8476054680012s and 990.899814648001s.
- Green: the exact-once check reported `total=245 unique=245 expected=245`.
- Green: the final CI/config contract run passed the helper’s deterministic assignment example.

### #2084

- Red evidence supplied by the issue brief: run `34216574622`, job `102029683849`, failed in Zipflinger with `java.lang.OutOfMemoryError: Java heap space`.
- Green contract: `task test TEST_FILE="spec/config/ci_runtime_lanes_spec.rb spec/config/android_taskfile_spec.rb spec/config/test_lane_metadata_spec.rb"` passed 16 examples with 0 failures and verifies all six variants, retained 4 GiB heap, `--max-workers=1`, and `--stacktrace`.
- Local execution limitation: `task android:ci` reached Gradle 9.7.1 but stopped before compilation because Android SDK location was unavailable (`ANDROID_HOME` and `mobile/android/local.properties` are absent). No one-worker OOM conclusion is claimed; a fresh Android runner remains authoritative.

## Other checks

- `task rubocop` passed all 1968 files with no offenses after the final review corrections.
- `task docs:build` passed with `No issues found`.
- `git diff --check` passed after the final report update.
- `task ci:check` was started once; local `/opt/homebrew/bin/actionlint` produced no output for 4m49s and was terminated. This is an environmental tooling limitation; the task did not reach its Node syntax checks.

## Self-review and concerns

- The browser workflow uses the existing Bash step only to consume the Node helper’s example IDs; it has no sleeps, retries, filename discovery, or weakened assertions.
- Timings are aggregated from the successful browser run by spec file and are preferred over dry-run setup times.
- Lighthouse upload and score-threshold steps remain unchanged; only selection, dependency, and aggregate result handling changed.
- Local Android execution and the actionlint check need fresh-runner verification. These are the only unresolved environmental checks.

## Hubble review response

- Red: the prior browser workflow used Bash process substitution, which could mask a failing Node shard helper. Green: the workflow now uses `set -euo pipefail` and direct output redirection; the executable contract rejects process substitution and the helper exits non-zero for invalid input.
- Red: the helper could fall back to `run_time` or zero when measured data was absent or invalid. Green: every example now requires a finite, positive timing keyed by example or file; executable regressions cover both missing and zero timings, and the committed manifest smoke check assigned 245 examples exactly once (`122 + 123`, `245` unique).
- Red: Lighthouse and path selection were previously checked through source text only. Green: the contracts now execute the classifier for representative UI and non-UI paths and execute the aggregate gate for success, failure, cancelled, missing, and unselected results.
- Red: the Android contract did not assert the existing heap or stacktrace settings. Green: the final contract asserts `org.gradle.jvmargs=-Xmx4096m`, `--max-workers=1`, `--stacktrace`, and all six existing variants.
- The original browser-sharding comment remains verbatim. The report makes no claim about removing compatibility links outside the aligned image setup.

### Final review-fix checks

- `task test TEST_FILE="spec/config/ci_runtime_lanes_spec.rb spec/config/android_taskfile_spec.rb spec/config/test_lane_metadata_spec.rb"` → 16 examples, 0 failures.
- `task rubocop` → 1968 files inspected, no offenses detected.
- `node scripts/ci/browser_shards.mjs --input tmp/browser-examples.json --timings scripts/ci/browser_timings.json --shards 2 --shard 1` → 122 examples, estimated duration `990.8476054680012s`.
- The same command for shard 2 → 123 examples, estimated duration `990.899814648001s`; combined output → `245` lines and `245` unique IDs.

## Next safe action

Bucky should run the required remote CI checks on the final diff, especially Android CI on a runner with the pinned SDK and the workflow/actionlint validation, then perform the independent review and publication steps.
