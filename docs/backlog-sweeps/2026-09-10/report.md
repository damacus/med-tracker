# Nightingale report: five test reliability issues

## Fix notes

fix(admin-users): wait for the visible membership-role option

The admin user role filter now scopes its Member selection to the visible listbox after the real role trigger opens it. Existing member and owner result assertions remain unchanged. Closes #2095.

fix(dashboard): wait for the visible direct-dose form

The all-family dashboard flow now scopes the direct-medication form to the open dialog and waits for the visible form after the real Take trigger. All-family selection and dose-recording assertions remain unchanged. Closes #2099.

fix(schedules): wait for the open medication combobox

The dose-less medication flow now waits for the visible medication listbox and selects its visible option after the real combobox trigger. The blocked next-step assertion remains unchanged. Closes #2140.

fix(test): refresh stale mounted dependencies before browser tests

Test preflight now hashes `package-lock.json`, refreshes a stale mounted test `node_modules` volume with `npm ci --ignore-scripts`, and launches the installed Playwright Chromium before the focused preflight spec. The verifier is a small Node support script called by the public Task command; shell entry points remain Fish. No browser-revision compatibility symlinks are used. Closes #2145.

fix(historical-dose): wait for the trigger-owned open dialog

Historical-dose browser coverage now waits for the real trigger, locates its nearest `ruby-ui--dialog` controller, and waits for that dialog to open. Escape, focus restoration, and dose-recording assertions remain covered. Closes #2212.

## Red evidence

- #2095 reproduced in the issue failure: the test selected the last `Member` label across hidden and visible content after the dropdown had not reliably opened.
- #2099 reproduced in the issue failure: the all-family direct-dose trigger was clicked, but the visible form was absent when the test searched for it; the captured page showed a selected all-family state and a faded Take control.
- #2140 reproduced in the issue failure: the test searched globally for `Dose-less medication` while the medication combobox was not reliably open.
- #2212 reproduced in the issue failure: `dialog[open][role=dialog]` was searched immediately after `Log a past dose`; the dialog was not yet open. The initial full-suite run after the first readiness change still failed this Escape example at the immediate open-dialog expectation, confirming that a page-global wait alone was insufficient.
- #2145 reproduced locally with `task test TEST_FILE=spec/system/admin_manages_users_spec.rb`: the mounted test dependencies requested Chromium revision `1234`, while the image installed revision `1243`, producing a missing `chromium_headless_shell-1234` launch path before browser assertions ran.
- The new #2145 Taskfile regression was first run before implementation and failed because no dependency-refresh or Chromium-launch task existed.

## Changed files

- `Taskfiles/test.yml` — adds the self-contained `test:verify-dependencies` task.
- `scripts/test_preflight.fish` — runs dependency/browser verification before the focused preflight spec and reports distinct failure status.
- `scripts/test_dependency_verification.js` — lockfile marker comparison, `npm ci --ignore-scripts` refresh, and real Chromium launch/close check.
- `scripts/test_capybara_browser.rb` — launches Chromium through the repository's Capybara Playwright driver path.
- `scripts/ci/policy.json` — maps both verifier scripts into the Rails CI suite.
- `spec/config/taskfiles_spec.rb` — regression coverage for task wiring, lockfile refresh, launch, and absence of symlinks.
- `spec/support/test_dependency_verification_harness.js` — executable current, stale, refresh-failure, and launch-failure verifier cases.
- `spec/system/admin_manages_users_spec.rb` — visible role-listbox scoping.
- `spec/system/dashboard_spec.rb` — open-dialog direct-dose form scoping.
- `spec/system/schedules/dosage_selection_spec.rb` — visible medication-listbox scoping.
- `spec/system/historical_dose_recording_spec.rb` — trigger-owned dialog readiness.

## Checks and results

- `task test TEST_FILE=spec/config/taskfiles_spec.rb` — 38 examples, 0 failures, including executable verifier cases and CI policy mapping; rerun after the stale-marker refresh-failure assertion correction.
- `task test TEST_FILE=spec/system/admin_manages_users_spec.rb:133` — passed twice.
- `task test TEST_FILE=spec/system/dashboard_spec.rb:208` — passed twice.
- `task test TEST_FILE=spec/system/schedules/dosage_selection_spec.rb:35` — passed twice; full file also passed with 3 examples, 0 failures.
- `task test TEST_FILE=spec/system/historical_dose_recording_spec.rb:46` — passed; full file passed with 2 examples, 0 failures after trigger-owned scoping.
- `task test TEST_FILE=spec/system/dashboard_spec.rb` — 6 examples, 0 failures.
- `task test TEST_FILE=spec/system/admin_manages_users_spec.rb` — 6 examples, 0 failures.
- `task test:verify-dependencies` — current marker path reported `Test node_modules match package-lock.json` and `Chromium launch preflight passed`.
- The same preflight also reported `Capybara Playwright launch preflight passed` from inside the test container.
- Stale-marker verification — a test-volume marker set to `stale` caused `Refreshing test node_modules from package-lock.json`; `npm ci --ignore-scripts` completed successfully, followed by the Chromium launch check.
- `task test:preflight TEST_FILE=spec/config/taskfiles_spec.rb` — dependency verification and focused spec passed; final output was `Test preflight passed: spec/config/taskfiles_spec.rb`. The final focused Taskfile check is 38 examples, 0 failures.
- `task rubocop` — no offenses in 1,966 files.
- `task docs:build` — passed, `No issues found`.
- `git diff --check` — passed.
- `task test TEST_FILE=spec/system/nurse_shift_medication_recording_spec.rb` — 1 example, 0 failures; this unchanged regression passed in isolation.
- `task test` on the final corrected diff — 6,231 examples, 0 failures, 1 pending (14m40s, exit 0). The five tranche-focused browser files and the verifier/policy spec pass independently; no change was made to nurse-shift code.

## Self-review

All four browser changes keep real user interactions and strengthen readiness waits and visible scoping. There are no sleeps, retries, removed assertions, or weaker assertions. The dependency check uses the current lockfile and refreshes only when the mounted marker differs; it launches both Node Playwright and the same Capybara Playwright driver used by system specs. The verifier adds no browser-revision compatibility links, and the existing Docker compatibility-link layer was not changed or removed. No application behaviour, dosing, authorization, shared RubyUI code, dependency versions, Docker images, or Compose volume layout changed. No comments were added, removed, or edited. The progress ledger was not changed.

## Concerns and next safe action

The local host's Fish process cannot access `/var/run/docker.sock`, so direct execution of `fish -c 'docker ps'` was not usable as independent evidence. The repository Task preflight succeeded with approved Docker access, and the Fish files passed `fish -n`. The remaining safe action is Hubble's independent review followed by Bucky's final integration check; Bucky owns the single commit, push, PR, and issue-status updates.
