# Delivery ledger

Resumed and completed on 2 October: the reviewed OAuth visibility fix was
installed, all after-split baselines and the final acceptance sweep passed on
fresh fixtures, and the final Rust, CI, docs and OpenSpec gates are green.
Publication (commit, push, stacked PR, CI watch) follows the plan's final
steps. Independent review was unavailable in this session; final review was
self-performed and is disclosed in the PR.

Paused at the user's request on 2 October until the rate limit resets. Agents
are stopped. Final application changes remain local and unpublished. The
[delivery plan](plan.md#resume-from-here) records the exact next steps.

Current status, 2 October: the household journeys through medication assignments
are published as five pull requests with passing CI. Rails remains operational.
The final set of fixes is still local: inventory and minor-user browser checks
pass, as do the repaired dose tests. All twenty-three sync tests and eight retry
tests also pass. The remaining compatibility checks passed too. The dose file
and its tests have now been split into smaller files. All seven relevant API
suites pass after the move, and independent review has accepted the split.
After the occurrence split, six API suites and all thirty-five dashboard tests
pass review. The schedule suite exposed two Rust compatibility bugs, tracked
in #2367 and #2368, alongside outdated test data and assertions. The repairs pass
all eleven active tests and six route checks both before and after the occurrence
split. Independent review has accepted it. The final current schedule run's
source and fixture hashes are runner-emitted only; that limit is recorded.
The invitation refactor is accepted after matching checks. Sign-in API checks
and all seven login browser tests pass before its refactor. It is now installed,
but matching checks are incomplete: a client-field visibility fix is installed;
the reviewed three-input-type visibility fix remains private and uninstalled.
The existing deterministic first-option race
test passed all four cases and six route checks. Reviewed extra assertions make
the zero-to-one option transition and unchanged stock explicit; their rerun remains.
The final changes need combined checks, review, publication and CI. The entries
below preserve the earlier decisions and results.

- 17:09 BST: resumed on `codex/rust-household-management-20261001`, based on
  published #2349. Starting acceptance 14/20. #2349 CI has no failures; Rust
  source and browser jobs still pending. Two-hour retrospective scheduled.
- Writer owns D first. E/F wait for preceding journey acceptance. Verifier is
  the sole runtime owner. Reviewer remains independent and read-only for code.
- 17:14 BST: #2349 CI run 36887533975 completed successfully, including both
  Rust source/build and browser jobs. The continuation baseline is published
  and green; no rerun or baseline repair is required for that PR.
- Verifier traced `api:openapi-dosages-acceptance` through the runner: `rails`
  refers to fixture generation; the `openapi-dosages` selector builds and tests
  captured Rust API. Prior scout target ambiguity is resolved by dispatch code.
- Independent #2349 review found medication unknown-error rendering remains a
  planned D gap, already excluded from accepted A/B/C; writer will fix it before
  task 4.3 acceptance. No extra People/Locations validation regression found.
- D tests authored before implementation: three browser HTTP cases exercise
  dosage persistence, tracked/null stock, exact custom units, rejected drafts,
  original stale/missing tokens, precision, CSRF and parent mismatch. A separate
  five-locale renderer regression covers unknown medication errors. Verifier
  owns baseline/RED execution; no new acceptance claimed until actual results.
- D baseline passed 14/14 Rust dosage HTTP tests. Verifier retained raw log,
  fixture identity and stable 302-path pre/post manifest. Full copied-source
  comparison raced cleanup; only the two critical new test/unit files were
  directly checked against the actual copy. Do not overstate that provenance
  or rerun the passing baseline solely for metadata. Next capture will compare
  a synchronously validated runner-equivalent snapshot with its emitted digest.
- Process ruling applied now: runtime freezes cover captured inputs; unrelated
  browser test authoring resumes at capture boundaries. Production changes
  still wait for relevant RED assertions. Keep one writer and one runtime owner.
- Independent reviewer resolved all three previously ambiguous #2347 contracts
  from Rails/OpenAPI/browser sources. Coordinator accepted the bounded rulings
  in plan.md; runtime proof remains G work. No premature baseline-test weakening.
- Snapshot preparation encountered known sandbox DNS denial before test execution.
  It is a setup failure, not RED. Verifier applies approved networking permissions
  to snapshot tasks too; avoid an extra source build solely for bookkeeping.
- D RED reached actual assertions: four new HTTP cases return 404 for the missing
  add route; the new medication unit rejects raw unknown diagnostics; focused
  dosage-locale integration also rejects raw diagnostics. The generic crate test
  stopped at its failing library test, so the existing TEST_FILE selector was
  used to run integration explicitly. Captured HTTP copy matched its validated
  manifest; web pre/post hashes matched. Writer now implements the affected
  browser routes and renderer. Existing dosage API extraction uses its green
  14-test baseline; no new acceptance is claimed yet.
- Native timer timestamp verified: created 17:07:45 BST; two-hour checkpoint
  19:07:45 BST (18:07:45 UTC). Root will run retro at this waypoint even if the
  busy-thread scheduler queues its prompt, then delete the one-shot automation.
- Independent static extraction review passed: 32 declarations retain original
  bodies after visibility-only normalisation, with policy, locks, transactions,
  inventory, audit and sync unchanged. Full D source review found no material
  security defect; browser permission, immediate use, aggregate/audit/sync and
  repeated-submit evidence still needed before acceptance.
- First GREEN fast compile caught a moved dialog-title String in medication.rs
  before Docker runtime acceptance. Same writer owns the correction. No new
  runtime fixture launched on that failed input; this is a compile failure,
  not a browser/test pass or a second failed implementation fix.
- Same-owner title clone fixed E0382. Fast API all-target check, selected new
  contract compilation and all Rust web unit/integration tests passed. Writer
  removed an unused extraction re-export and is adding the independent review's
  missing forged/member, audit/no-write and JS-enabled immediate-dose checks.
- Two intermittent Sol 6.1 capacity errors were recovered by resuming the same
  writer. No ownership change. One dosage baseline runtime job had already
  launched when the final-capture hold arrived; allow its healthy immutable run
  to finish and certify only that capture. Further captures wait for the writer's
  explicit final strengthened-test notice. Do not infer source freeze from an
  agent's capacity error or quiescent status.
- Rails source policy review confirms ordinary manage-granted members cannot
  create/edit dosage options; the dosage policy delegates to medication update,
  not the broader medication create rule. Existing Rust owner/admin restriction
  matches that case. Runtime member-denial proof still pending.
- Bounded E source review resolved tracked-stock adjustment: parent endpoint
  changes only the parent and must not be presented as reconciled option stock.
  Existing option PATCH is the coherent path. Filed dropped scalar audit reason
  as #2350, verified labels `rust,bug`, and scheduled its RED/GREEN fix in E.
  No new endpoint or invented reason field is authorised.

## Waypoints

| Journey                       | State                                              | Evidence                                                 |
|-------------------------------|----------------------------------------------------|----------------------------------------------------------|
| D dosage management           | Published in #2351; CI passed                      | Writer report, independent review and verification queue |
| E stock                       | Accepted and published in #2357; CI running        | Stock ruling, stock review and verification queue        |
| F assignments and schedules   | Checking existing API behaviour before file splits | Assignment brief and treatment rulings                   |
| G parity and final acceptance | Waiting for F                                      | Plan                                                     |
| Two-hour retro                | Completed; local improvements applied              | retro.md; native timer deleted                           |

- 18:26 BST: D core HTTP passed 4/4. Browser retry passed 20/20 after a
  test-only pagination repair; all assertions were retained. A separate fresh
  fixture passed the grant-mutating permission case 1/1 and browser cases 20/20.
  Independent review verified the actual assertion logs and translated dialogs.
  Full `ci:rust-port` is running on frozen source; acceptance remains 14/20
  until its result and final independent review. No product retry was needed.
- 18:27 BST: full `ci:rust-port` passed on frozen authored inputs; independent
  requirements and quality/security review passed. D/4.3 is accepted: 15/20.
  Publish this completed journey before starting E. Two presentation findings
  remain explicitly queued for E/shared integration. The actual two-hour retro
  remains at 19:07:45 BST; it has not occurred yet.
- D published as ready [PR #2351](https://github.com/damacus/med-tracker/pull/2351),
  commit `fb276edc0ad5b45ccfe14a093b95dbc9c08b7180`, above #2349. Documentation
  build and diff checks passed with unchanged Markdown. Signing used the approved
  single-command fallback after the 1Password agent failed. Branch synchronised;
  no merge or deployment. E now owns `codex/rust-household-stock-20261001`.
- 18:37 BST: E test-only packet frozen: four HTTP cases, including the actual
  dropped audit-reason regression, plus ten five-locale desktop/mobile stock
  workflows. Existing Rust baseline and actual RED jobs are queued with the
  sole verifier; reviewer checks setup and contracts independently. Product
  implementation waits relevant observed failures. Acceptance remains 15/20.
- 19:07:57 BST: actual two-hour retrospective started, verified with the live
  clock. D is published; E has relevant RED and is being implemented. Fixture
  prechecks, narrower capture holds and focused reruns now govern remaining work.
  The timer was deleted after the checkpoint. Continue E/F/G; no deadline stop.
- Stock runtime checks passed: 17 existing API tests, 13 new API checks, 41
  selected browser cases, the separate permission case and selector check, and
  seven prior medication browser regressions. Independent review confirmed the
  state checks and reviewed desktop/mobile screenshots in all five languages.
  The full Rust gate found three lint errors in the new stock adapter; the same
  writer is fixing them. Stock remains unaccepted and unpublished until the
  final gate and review pass. The original denominator remains 15/20.
- D CI passed on its published source. One Rails system job passed on its single
  reproduction; the original intermittent failure remains tracked as ruby/bug
  in #2354. No Ruby product change or production action was made.
- The user’s plain-language feedback is applied to progress updates and report
  openings. The four additional mixed-purpose file findings are recorded in
  remaining-module-review.md and #2348 for the final refactor stage.
- 20:33 BST: the final Rust gate passed after four behaviour-preserving lint/type
  repairs. Independent requirements and quality/security review passed. Stock
  management is accepted, completing 4.1 and 4.4: 17/20 original tasks. Check
  the final documentation, publish this journey, then hand off assignments.
- E is published in ready [PR #2357](https://github.com/damacus/med-tracker/pull/2357)
  above #2351. Feature commit is `dd9e84ac472bdd060b4fc1ade30ca88cef323a0a`;
  Markdown-only follow-up is `bbace56eeff0142a18325b686264d390d9023d40`.
  Documentation and exact CI Markdown checks passed. Branch was pulled, pushed
  and synchronised. The authorised signing fallback was used after 1Password
  failed; signing configuration remains unchanged. No merge or deployment.
- F owns `codex/rust-household-assignments-20261001`, based on the published
  stock tip. Its three existing API test targets compile. Assignment and
  schedule runs found outdated allowed-field lists; the approved repair keeps
  optional fields optional, checks their types and still rejects unknown keys.
  The repaired assignment and schedule checks passed 11/11 and 10/10; the
  unchanged pause checks passed 10/10. Independent review confirmed the test
  changes preserve the documented contract. New browser tests are being
  reviewed before implementation.
- Replies were posted to all six dosage review threads on #2351. Five addressed
  findings were resolved after their fixes or explanations were verified. The
  501-option limit remains open in #2353 for the final compatibility stage.
  Stock CI finished with 18 successful checks and five inapplicable checks
  skipped. Issues #2350 and #2352 are closed as implemented and verified in the
  published pull request. The pull request remains open; nothing was deployed.
- The new stock review confirmed that the existing 500-option limit also affects
  medication pages, and found a browser stock-loss gap when all options have
  blank stock. The latter is tracked in #2358 with rust and bug labels. Both
  remain in final compatibility work, with their review threads open. The
  claimed 255-character audit limit was checked against the schema and rejected;
  its explanation was posted and the thread resolved.
- The ten new browser tests reached the missing assignment link and failed as
  expected. The combined command did not run the new HTTP tests: Task variables
  were not exported to the runner. Correct the invocation with explicit exports
  and verify its selected target and test count. Do not repeat the valid browser
  failure merely to repair that separate evidence gap.
- The missing assignment pages were confirmed by nine failing HTTP tests. The
  new screens now compile and pass lint checks. The three large assignment,
  schedule and pause files have been split into smaller modules; all 31 existing
  API tests still pass after those moves.
- Fourteen form tests are ready to check successful saves, optional fields and
  required end dates. Review confirmed their setup matches the API. They now
  test the new screens before we fix the remaining form-submission errors.
- The test-command repair passes its focused tests, including conflicting shell
  settings and filters containing spaces or quotes. The next real run must show
  that the requested fourteen HTTP tests actually run. This fixes #2359 once the
  change is verified and published.
- Progress reports now explain the user-visible change and remaining checks in
  plain language. Detailed commands and test records remain available separately.
- All fourteen requested form tests ran, confirming the command repair works in
  the real test environment. Nine passed. Five found the expected problems:
  blank optional fields prevented saves, unchanged blank cycles prevented edits,
  and a missing end date changed the unused retry key. The submission fixes are
  now underway; browser checks will follow the corrected forms.
- The corrected forms passed all fourteen HTTP tests. The ten browser journeys
  stopped at the same mistaken expectation: resuming a future schedule was
  expected to make it active today. The schedule correctly remained ineligible
  by date. The test now checks restored status and unchanged dates, dose and
  history; independent review approved that correction. Pause/resume conflict
  tests run next, followed by the focused browser retry.
- Eighteen pause/resume tests ran. Both public API compatibility checks passed;
  sixteen browser protection or form-retention checks failed. Out-of-date and
  incomplete forms were allowed to save. Three rejected saves reached the right
  error but their returned forms lacked the original treatment identity. The
  browser protection and retained fields are now being fixed. The public API
  contract remains unchanged.
- The pause/resume fix passed all eighteen HTTP tests. The ten corrected browser
  journeys passed across all five languages on desktop and mobile, including
  saved edits for each of the seven schedule types and pause/resume history.
  Remaining assignment checks cover incomplete retry information, real dose
  recording, daylight-saving/taper display and revoked access. Final review,
  quality checks and publication still follow.
- Final core assignment checks passed: 37 HTTP tests and 12 browser journeys,
  including actual option-stock consumption, the current taper dose and rejected
  recording while paused. The separate calendar checks found a dashboard display
  defect: today's taper cards show the base 1.25 ml rather than the current
  0.75 ml step. Issue [#2360](https://github.com/damacus/med-tracker/issues/2360)
  has `rust` and `bug` labels. Recording already uses the correct amount; the
  display fix and its precise regression test remain part of assignment work.
- Revoked-access checks passed in their corrected fixture, followed by seven
  existing medication browser tests. Stock that was outside the member's access
  correctly returned not found in the earlier test; access policy was unchanged.
- Review found a second taper form bug: the available time fields write nested
  values that neither Rails nor Rust uses to decide when doses are due. The form
  lacks controls for the effective schedule times. Issue
  [#2361](https://github.com/damacus/med-tracker/issues/2361) has `rust` and `bug`
  labels. Fix and verify those controls before accepting assignments; preserve
  existing configuration and API recurrence rules.
- Both taper fixes now pass focused desktop/mobile checks. The final candidate
  passed all 37 HTTP tests and 12 browser journeys again, with 102 verified
  screenshots. Independent requirements review passed. The full Rust gate
  passed on unchanged source. Original journey requirements are 20/20; final
  documentation checks and publication follow. The remaining stock bugs,
  compatibility, five refactors and runtime CI work are still required.
- Assignment and schedule work is published in
  [PR #2362](https://github.com/damacus/med-tracker/pull/2362), at commit
  `9584ef7c0c55173d0b9f51bd26da6c3b625b52f3`. The changed-file Markdown check
  passed for all ten documents after three presentation fixes; the documentation
  build passed. Remote checks are running. The final stock regressions and
  required household runtime CI checks are being added on a dependent branch.
- Assignment CI run `36941130246` succeeded: sixteen jobs passed and five
  inapplicable jobs were skipped. Issues #2359, #2360 and #2361 are closed as
  fixed and published. PR #2362 remains open. The final branch also covers the
  confirmed minor-viewer person-page bug in #2363 and clearer linked-dose edit
  guidance. Two taper review warnings were checked, explained and resolved as
  false positives; their suggested policy changes were not applied.
- The first final-work run stopped at invalid test setup: medication creation
  rejected a blank reorder threshold. The helper now uses the required value
  and records API error details. The corrected four-test run reached the stock
  assertions: both rejection controls passed; medication-stock removal failed
  with 422 instead of 303, and a 501-option medication page failed with 503
  instead of 200. These reproduce #2358 and #2353 before production changes.
  The copied application matched all 370 checked source paths. Raw output is
  preserved at `/private/tmp/household-g-20261002/G-INITIAL-RED-002/runner.raw.log`,
  SHA-256 `40485820728b23bd6a4eabd37a977235e38a1fd3357e8c351cad35e70dc6aa12`.
- The stock fixes now pass all four focused HTTP tests and six existing browser
  checks. A medication with 501 dose choices loads successfully. When every
  choice has blank stock, removing stock uses the medication's recorded balance;
  insufficient stock and conflicting retries leave records unchanged.
- Editing an assignment now has verified safeguards: leaving the dose choice
  blank keeps its existing link, an incompatible dose is rejected, and choosing
  a matching replacement saves correctly. Both HTTP tests and all six route
  checks passed. Import checks follow before the large import file is split.
- A minor can read an authorised person page and assignment history while
  schedules remain restricted. The focused HTTP test passed in all five
  languages. Two rendering tests also passed, including wording that avoids
  claiming there are no assignments when schedules cannot be shown. Actual
  desktop and mobile browser checks remain pending.
- The large-household check passed, covering more than 500 visible medications,
  assignments and schedules. Person pages and treatment choices loaded complete
  collections; all six existing browser route checks also passed.
- All five import checks passed before refactoring: two portable-write tests and
  three export/import tests, including public readback and rejected access to
  another person's data. The import file is being split first. Repeat those
  same checks and review the move before starting another large file.
- CI selection checks passed after adding the combined final household target
  and a separate minor-user browser row. These checks verify which tests CI
  selects; the two new browser groups still need actual runtime acceptance.
  Review caught invalid browser-test setup before running it: teardown must
  wait for the tests, stock errors must use the existing translated message,
  and the minor viewer needs explicit preparation in the disposable fixture.
- Independent review passed the import split: all 49 routine bodies and eight
  constants or types are preserved, with no comment changes. The first compile
  found two fields needing access between the new private modules. Those fixes
  are in place and compilation passes. Formatting changes are recorded
  separately; lint and the five post-split import tests remain pending.
- The import split passed lint and all five existing import tests after the
  move, matching the pre-split results. Independent review checked both test
  logs and matching application copies. This one refactor is accepted; four
  other large files still need their own checks and review.
- Final inventory acceptance passed all seven HTTP tests and ten browser
  journeys across five languages on desktop and mobile. Rejected saves kept
  entered values and stock unchanged, successful removals saved correctly,
  retries did not duplicate changes, and horizontal scrolling made the final
  mobile navigation link usable. Minor-user browser checks remain separate.
- Minor-user acceptance passed one HTTP test and ten browser journeys across
  five languages on desktop and mobile. Authorised person pages and assignment
  history remain available; schedules stay restricted. All twenty screenshots
  are preserved. A later source snapshot matches the digest reported by the
  run; the original temporary application copy was not retained.
- A stock adjustment with a 300-character reason saved correctly. An
  8,192-character reason returned a server error and left stock and history
  unchanged. Issue #2365 records the confirmed Rust failure and the matching
  Ruby code that needs separate verification. The agreed Rust fix preserves
  the complete reason in the audit record and uses a short event label when
  the full label exceeds the internal storage budget. Implementation and
  acceptance are in progress; no Rails fix is claimed.
- The Rust audit fix passed six tests, including the long reason that previously
  failed and byte-boundary checks with ordinary text and emoji. The existing
  short-reason stock test also passed, as did both sets of browser route checks.
  Independent source review passed. The full reason remains in the audit record;
  stock, history and sync each record one successful change. Publication remains
  pending, and the Ruby follow-up in #2365 remains open.
- Stock-choice compatibility passed both HTTP cases and all six route checks.
  Matching stock now follows location name, then medication ID. The tests also
  prove that recording a paused dose or exceeding ordinary or tracked stock is
  rejected without changing stock, dose history, versions or sync records.
  Two old expectations were corrected to match the documented rule that positive
  stock stays visible while saving rechecks the quantity. Original source
  manifests match the tested copy. People pagination is the next check.
- People pagination passed its existing regression and all six route checks.
  Oversized page requests now use a maximum page size of 100. Invalid inputs,
  access restrictions and the behaviour of other resources remain covered by
  passing controls. Independent review accepted the scoped fix and matching
  source evidence. The dose-recording baselines follow before its file is split.
- Before splitting dose recording, the existing stock-write, sync and retry
  suites passed: six, twenty-three and eight tests respectively, each on its
  own fixture. The two dose-mode tests also passed after repairing their setup.
  The larger dose suite exposed missing setup values and a disagreement over
  the error returned for an unknown source type. Rails treats that source as
  missing. Two new tests now check this response through direct recording and
  sync, while keeping malformed-input errors and unchanged-record checks.
  Their first run found the expected error mismatch after all malformed-input
  and unchanged-record checks passed. The small response fix then passed both
  focused tests, six direct-write tests and their route checks. Independent
  review accepted the focused fix on retained matching source manifests. The
  wider dose checks and the file split remain pending.
- The larger dose suite then passed eight tests and failed six, with one
  pre-existing test ignored. Review traced three failures to occurrence audit
  labels, two to Rails error responses, and one to a pagination expectation
  that conflicts with the documented API limits. The writer is preparing
  narrow fixes; the reviewer will check them before installation. Issue #2347
  now records these results and the remaining compatibility work. The verifier
  is capturing the other known test failures on the same frozen application
  while this patch is designed. The dose split remains held.
- The repaired dose suite now passes all seventeen active tests. The existing
  timestamp-response test remains ignored. The checks cover audit labels,
  paused doses, numeric input, pagination and empty stock. Rejected requests
  leave stock and dose history unchanged. Both focused source-error tests,
  all six direct-write tests and their route checks also pass. Independent
  review accepted the fixes and confirmed that the tested source matches the
  current files. Sync and retry checks are running before the large dose file
  is split. The final tranche has not yet been published.
- The remaining compatibility checks passed: three sync-event tests, nine
  medication API tests and three read-response tests, with six route checks
  after each. Independent review accepted the results and matching source.
  We then installed only the reviewed dose split: a 164-line entry point with
  twelve smaller source files, and the same test target divided into eight
  files. All twenty-two installed files matched the reviewed proposal before
  formatting. The verifier is checking compilation and rerunning the same
  tests. The occurrence, invitation and authentication splits remain proposals.
- The dose split is accepted. Its entry file is now 217 lines, with original
  public definitions retained and no lint suppression. Formatting, API
  compilation, lint and unit checks pass. The seven API suites passed on
  separate fixtures: seventeen active dose tests, two source-error tests, six
  direct-write tests, two stock-mode tests, two stock-choice tests, twenty-three
  sync tests and eight retry tests. Each also passed six route checks. The
  existing ignored timestamp case and all eighteen dose test names remain
  unchanged. Independent review verified code preservation and matching tested
  source. The first run retains its reported fixture hash; the other six also
  have independently checked fixture hashes. Occurrence checks now follow.
- The occurrence checks passed before its move: ten API tests, two sync-read
  tests and thirty-five calendar/dashboard browser checks. The accepted dose,
  sync and retry results also cover the unchanged code at this point. The
  occurrence entry file has now been reduced to 117 lines with thirteen
  smaller files. All fourteen installed files matched the reviewed proposal.
  Compiler and after-move tests are running; the last two splits remain private.
