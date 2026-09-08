# PR 2126 CI repair

Run 34212412054 failed in two distinct places:

- Browser shard 2: successful-test screenshot helpers hard-code `/app/tmp`,
  which is absent on the native GitHub runner. Keep the overflow assertions;
  save artifacts through the configured Capybara screenshot directory.
- Production-image smoke: Fish installation stopped during apt metadata update
  because the unrelated Microsoft package repository returned HTTP 403. The
  application smoke test never started. Verify this separately on the next run.

Nightingale owns the bounded screenshot helper fix; Sol reviews it independently.
Root verifies the affected browser spec and Ruby lint, then commits and pushes
to the existing PR and follows the remote checks. No application code changes
or dosing changes are needed. Existing CI failure is the regression evidence.

Status: implementation, independent review, focused browser tests, and lint
complete; commit, push, and remote CI follow-up remain.

The two helpers in `spec/system/mobile_overflow_spec.rb` now resolve relative
filenames beneath Capybara's configured `Capybara.save_path`, create that
directory when needed, and use the Playwright driver screenshot API. This
preserves the existing width guards and screenshot filenames while removing
the native-runner dependency on `/app/tmp/capybara` and retaining all
overflow, focus, menu, and privacy assertions.

Focused verification used:

`COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_overflow_spec.rb`

Result: 9 examples, 0 failures. `git diff --check` also passed. The test run
created the expected screenshots under the configured local Capybara save path
(`tmp/capybara`); no workflow or production files were changed.

Full Ruby lint inspected 1,824 files with no offenses. Documentation build
passed.

The production-image smoke from the same CI run passed on retry (run
34212412054, attempt 2, job 102032118314), so the unrelated Microsoft apt 403
did not require a workflow change.
