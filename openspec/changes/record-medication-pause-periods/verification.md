# Pause-period completion evidence

Implementation is under review in a lower-first feature stack:

- [Portable history #2147](https://github.com/damacus/med-tracker/pull/2147)
- [API and sync #2148](https://github.com/damacus/med-tracker/pull/2148)
- [Web forms and history #2149](https://github.com/damacus/med-tracker/pull/2149)
- [Android controls #2150](https://github.com/damacus/med-tracker/pull/2150)

The separate native implementation is published as
[iOS PR #24](https://forgejo.ironstone.casa/damacus/med-tracker-ios/pulls/24).

## Existing downstream behaviour

`task test:preflight` passed: 31 examples, no failures.

Focused reminder eligibility/jobs, report index/health history, adherence streak and
missed-dose-pattern specs passed: 74 examples, no failures. Their existing implementation
already consumes pause intervals; the stale OpenSpec checkboxes were reconciled.

## Web

Initial pause/history/preload checks passed: 43 examples. Existing request/card regression
checks passed: 52 examples. Independent review identified lost history after a dose Turbo
refresh. Regression tests reproduced three failures, then all 42 request examples passed
with the preload fix. Independent re-review approved the fix.

Manual browser verification used this worktree's development fixture household, signed
in as `admin@example.com`. Pausing John Doe's Movicol schedule with Out of supply and
"Waiting for the next delivery." displayed the context on its card. Resuming retained
the reason, note, pause/resume times and Admin Doe actor in expanded pause history.

Screenshots:

- `docs/screenshots/issue-1981/pause-form-mobile.png`
- `docs/screenshots/issue-1981/pause-history-desktop.png`

## API, portability and sync

API/OpenAPI/shared lifecycle checks passed 138 examples. History ordering and retired-source
visibility checks passed 18 examples. After reproducing cached-response disclosure on revoked
person grants, 39 API examples passed with fresh authorisation before replay.

Portable v2 regression checks passed 93 examples. The final 18 focused examples include an
encrypted cross-household round trip, unchanged native provenance and rejection of imported
changes to native history. Imported history does not attribute original actions to the importer.

Sync checks passed 20 examples, including complete-batch rollback, current grant checks on
replay, and one new result plus one replay for simultaneous creates.

## Android

The full Android `task ci` passed: unit tests, lint, phone/Wear builds, release security,
Wear boundaries and API checks. Three pause Compose emulator tests passed. Independent
review approved retry identity, mutation ordering and discovery across all source pages.

Screenshot: `mobile/android/docs/screenshots/medication-pause-phone.png`.

The integrated review found that real paused API sources have `active: false`.
Resume now uses pause state and the current period. New gateway, controller and
Compose regressions cover both schedules and direct assignments. All 17 focused
pause unit tests and six emulator UI tests passed; Android lint passed. The final
UI run targeted the emulator because an unrelated connected physical device could
not create the Compose test hierarchy.

The full Android `task ci` passed again after the Resume correction, including
phone/Wear tests, lint, all build variants, release security, Wear boundaries and
API drift checks. The command used the installed JDK 17 and Android SDK explicitly.

## Final review corrections

Cached invalid sync batches now return the original 422 response on retry.
Successful cached responses still require fresh authorisation. The regression
covers invalid reasons, notes, timestamps and missing sources.

Hosted household archives now embed portable v2 and include pause history. The
outer household archive format and explicit portable v1 endpoint stay compatible.
The exported schedule and pause records restore through the supported importer
boundary. Full household archive restoration is not established: the existing
importer rejects the `health_events` collection, including an empty array. This
separate gap is tracked in [#2146](https://github.com/damacus/med-tracker/issues/2146).

The final Rails correction checks passed 66 examples, followed by 42 examples for
the exported-source round trip and adjacent replay safety. RuboCop passed across
1,853 Ruby files. A previous full Rails run completed 5,691 examples with three
failures and one expected OIDC pending example; the three failures were fixed.
The final full run passed: 5,696 examples, zero failures and one expected OIDC
pending example, in 15 minutes 53 seconds. After stack integration, the complete
feature diff matched the tested diff byte for byte before provenance updates.

## Verification environment follow-ups

The test task wrapper skipped its requested second command because an included
task used `run: once`. This is tracked in
[#2144](https://github.com/damacus/med-tracker/issues/2144).

A rebuilt image retained stale Playwright dependencies in its mounted test
volume. Refreshing that isolated volume with the frozen npm lock restored browser
tests. This is tracked in [#2145](https://github.com/damacus/med-tracker/issues/2145).
Neither workaround changed application or shared tooling source.

## Integrated checks and remaining gates

The final independent review approved all four corrections: Android paused-source
Resume, cached invalid sync replay, iOS V4 store hydration and confirmed iOS pause
handling after refresh failure. No further material regression was found.

iOS focused native checks passed 33 tests, with a separate 11-test model regression
run. Four pause UI tests passed on iPhone and four on iPad. The final iPad
accessibility rerun passed reason selection, empty optional note submission and
retained input after failure. Screenshots cover light/dark forms, history, paused
treatments and the largest accessibility text size. The final full iOS `task ci`
passed: API 25 tests, native unit 158 tests, UI 39 tests, lint, contract regeneration,
simulator build and unsigned release archive. The published iOS commit is
`8aa896fffebb1b1c7b5873bd0a05c79086dcaa5f`.

The full iOS gate exposed two test-only issues after production review. The shared
refresh test now waits for both readers to join before releasing its transport,
and still requires exactly one request. At the largest iPhone text size, the pause
test scrolls the native picker menu before selecting Other. The affected sync
suite passed 16 tests, and the four pause UI tests passed together with selection
and submission assertions intact. Production code did not change for these fixes.

Brakeman passed with zero errors and zero warnings. Documentation builds and strict OpenSpec
validation passed. The full Rails suite and final RuboCop checks passed.

GitHub's initial non-browser jobs completed the examples but reported API branch
coverage of 586/655 (89.47%), below the unchanged 90% threshold. Eight additional
behaviour tests passed 51 focused examples and hit ten previously missed branches.
The final full non-browser suite passed 5,478 examples with zero failures and one
expected OIDC pending example. API branch coverage is 596/655 (90.99%); overall
line coverage is 94.77% and branch coverage is 79.06%. All thresholds remain
unchanged. Final RuboCop passed across 1,854 files. Stack integration reproduced
the tested tree exactly and retained the same OpenAPI source revision, so the
verified native contract pins remain valid. Remote CI reruns on the refreshed
API and dependent PR heads; those results are distinct from the passing local
coverage gate. Forgejo PR #24 also has a pending remote pull-request check.

All five PRs are open for review. Publication does not establish a deployed
feature or a signed/distributed native build. The separate health-event restore
gap and test-tooling follow-ups remain tracked in #2144, #2145 and #2146.

No merges, deployments or native distribution are authorised by this implementation.
