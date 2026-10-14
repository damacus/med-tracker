# Migration progress

Updated 14 October 2026. Complete every slice in [the plan](plan.md).
Foundation is accepted. The verified baseline PR #2451 is merged; all remaining
migration areas stay in scope. No live deployment has occurred.

## Current delivery

### s05-i01 — API contract completion, 13 October 2026

Current candidate: `codex/s05-i01-http-rate-limits`, based on merged care revision
`672dbc0d1520ee586ce625f0dbfec7fb3eba0544`. Full CI and independent review pass;
publication is next. The current inventory has 122 operations after the merged
review leases: two partial and 120 unverified. Historical snapshots below retain
their original operation counts and publication states.

The owner approved completing all 119 documented API v1 operations using
`rust-testing`, upstream Loco guidance and area-based `gh stack` PRs, each at most
2,000 added plus deleted lines against its immediate base. The detailed agreement
is in [the API slice](slices/05-api-integrations.md).

- Baseline: refreshed `main` at `714eaac7a2e981451265edbab1b295096ff12bab`,
  including the recent test-registration, isolated AI-configuration and shared
  fixture-helper fixes. The stack was rebased with `gh stack rebase`.
- Active branch: `codex/s05-i01-contract-harness`, stack rooted on `main`.
- Coordinator: current Codex chat; sole source writer: `/root/api_writer`;
  verification/resource owner: `/root/verifier`.
- Current work: exhaustive operation mapping and Loco-native request acceptance,
  preserving isolated PostgreSQL 18 fixtures and restricted runtime roles.
- Independent reviewer: Devin CLI `swe-2-max`, session `cool-actress`;
  packet and output retained under `/private/tmp/s05-i01-review/`. Its first
  attempt stopped at CLI read permissions without a verdict. Devin accepted the
  corrected source with framework source supplied directly. The same-session
  follow-up also accepted the fixture registration correction after rebasing.
- Focused checks: inventory/harness 12 passed, JavaScript dispatcher 16 passed,
  formatting, documentation and whitespace passed. The complete API group ran
  thirteen tests: twelve passed and its completeness gate failed as intended.
  All 119 operations still require
  complete behavioural evidence. Loco registers 118 documented operations; the
  missing route is `POST /api/v1/households/{household_id}/push_subscription/test`.
  Logs are under `/private/tmp/s05-i01-evidence/`. Coverage is recorded below.
- Root CI on signed source `5f12b5b86a8ad650e59246f52f08783f98184b3d`
  (job `62949`) failed with two transport errors: a five-second app-token GET
  timeout and a mobile recovery HTTP parse error. The other 483 care API and
  661 browser cases passed. Both failed cases passed individually without code
  changes; their causes remain unproven. Original logs and browser trace remain.
- Final root CI passed on signed source
  `11ea37b402fa1583dc35ec768b4e384e5d1bad47`: `task --concurrency 1 ci`, job
  `14255`, exit zero. All 662 browser cases passed; care API reported 486 passed,
  zero failed, one ignored and fourteen filtered. Documentation and whitespace
  checks also passed. Full log: `full-task-ci-11ea37b4.log` in the evidence folder.
  This records a successful serial run, not a proven cause or fix for the earlier
  transport failures. The final commit adds only this verification record.
- Published [PR #2600](https://github.com/damacus/med-tracker/pull/2600) against
  `main` at signed head `b56d4b486b85ced3bbe8e22ffd9bec265c722bb3`: 1,099 additions
  and nine deletions. Full API acceptance remains pending.
  No merge or deployment.
  The owner's GitHub update subsequently merged `main` into this PR at
  `2808f38ce508f2606b6d2d0787301961bffd85d7`, preserving our source and adding
  merged PR #2599's unit coverage and property tests. The local stack now
  preserves that merge. Earlier local CI is evidence for its recorded source;
  [Hosted CI run 38412852801](https://github.com/damacus/med-tracker/actions/runs/38412852801)
  passed on this exact updated revision. The hosted checksum finding was answered
  and resolved after independent same-session Devin adjudication accepted the
  full-document provenance guard; no source change was needed.
- Review corrections: duplicate schema/header substitutions and handlers from
  another route now fail explicit regression tests. Both tests failed before the
  guards were corrected. Final focused log: `green-review-metadata-guards.log`;
  final expected incomplete gate: `review-fixes-expected-incomplete.log`, both
  under `/private/tmp/s05-i01-evidence/`. Same-session re-review accepted both.
- Inventory now includes explicit schema/header/handler references and gaps for
  every operation. Middleware proof uses two requests and persisted source data,
  avoiding reliance on a timed request burst. The initial signed snapshot is
  `c7edacd480b103bfe551884b3d5fc68e61ccf568`; the corrections above were verified
  after that snapshot and remain on the same branch. Rebased source is
  `efb13f2473e9b6c2efa0d53fea914edd4ebcc397` before the fixture registration fix.
  The missing root helper first failed compilation; its two-line registration
  then passed all twelve focused cases, formatting and documentation checks.
  The publication gate recounts the final diff against the immediate base.
- Next action: extend report-only coverage on
  `codex/s05-i01-coverage-baseline`, based on PR #2600, coordinating with #2583.
  PR #2599 closed #2583's unit-coverage scope. This branch extends its existing
  Task/job to integration coverage and reproducible evidence; it adds no duplicate
  coverage pipeline. Overlapping local work was preserved before reconciliation.
  The owner approved correcting two OpenAPI omissions: capability rate-limit
  responses and account OAuth insufficient-scope responses. Apply each correction
  in its owning area, preserving enforcement and leaving native pins unchanged.

Main subsequently advanced to `60e7de5d` (PR #2604), adding pure unit tests.
The harness branch preserves that ancestry and corrects the new blanket
TestServer prohibition to match the approved full-router acceptance harness.
Devin session `cool-actress` accepted this guidance and withdrew an inaccurate
in-process transport suggestion after checking installed axum-test 17.3.0. Both
agent guides are synchronized at signed harness head `d8201065`; publication
and updated-head hosted checks remain pending. No upstream tests were removed.

Coverage extension on `codex/s05-i01-coverage-baseline`:

- Base: PR #2600 branch at `d8201065910fbe79f276b514970399919a67eb51`, including
  PR #2599's unit-coverage setup. Extend its `test:coverage` command and existing
  CI job to run unit, request-harness and care API tests using owned fixtures.
- Preserve the report-only policy. Test/tool failures still fail the selected
  CI job; no line-percentage threshold is introduced.
- Coordinator retains documentation and stack ownership; the existing writer
  owns implementation/tests, and the verifier owns provisioning and live jobs.
- Red evidence covers the missing Task boundary, CI aggregate failures,
  contaminated metadata environment and shared instrumentation paths. Corrected
  runner checks pass 24 cases; CI policy checks pass 137. Installed prerequisites
  are cargo-llvm-cov 0.9.1 and matching Rust 1.99 LLVM tools.
- Each run now owns its target, build intermediates and report directory. The
  report records source revision, dirty state and a source fingerprint, rejecting
  mutations during measurement. Failed runs cannot publish a complete manifest.
- Independent Devin SWE-2 Max session `living-pick` identified an ambient coverage
  directory override and verification gaps. Corrections passed focused checks
  and the same reviewer accepted the final source: tool overrides are stripped, version matching
  is exact, malformed or padded output paths fail before provisioning, failed-output checks
  cover the real run directory, and the redundant test Task alias is removed. Its
  packet and output are under `/private/tmp/s05-i01-review/coverage-*`.
- The host's actionlint 1.7.12 hit a Darwin deadlock. The unchanged workflow check
  passed with the maintainer's merged fix, commit
  `011a6d15e749bb3f2d771eed9c7aa0e7e3e10ee7`, built only under `/private/tmp` and
  selected through a scoped PATH. ShellCheck remained enabled; global tooling
  and repository checks were not weakened. Logs retain the timeout and passing run.
- First instrumented candidate `89d04ce6` passed 45 unit tests, then job 34708
  failed because cargo-llvm-cov 0.9.1 rejects combining `--no-report` with
  `--no-clean`. No request/database coverage or complete report was credited;
  owned run resources were cleaned. A regression reproduced the CLI rejection.
  Removing the redundant `--no-clean` flags preserves accumulation through
  `--no-report`; 24 runner and 137 policy checks pass after the correction.
  Devin accepted this correction in the same `living-pick` session. Repeat
  measurement and cumulative CI passed as recorded below.
- Existing hosted unit coverage at `2808f38` measured 1,931 of 57,239 application
  lines (3.37%), including zero API-controller lines. This `--lib` baseline is
  preserved under `/private/tmp/s05-i01-evidence/hosted-unit-coverage-2808f38/`.
  It does not measure the full suite.
- Combined coverage passed on signed `ca314adb`, before the latest main update:
  job 75191 exited zero with 45 unit, 12 request and 500 care API tests passing.
  The request completeness gate and fixture hook remained intentionally ignored.
  LCOV and a complete manifest are preserved in
  `/private/tmp/s05-i01-evidence/coverage-baseline-ca314adb/`; the manifest records
  the unrelated preserved Serena change and an unchanged source fingerprint.
  Application line coverage is 36,202/57,242 (63.24%); the API-controller
  directory is 6,717/7,287 (92.18%), the OAuth-controller directory 10/344 (2.91%),
  and models 27,775/38,185 (72.74%). These directory metrics do not establish
  public API acceptance or meet every skill target. The log proves all three
  selections ran; LCOV does not label each selection independently.
- Docker pool exhaustion delayed measurement. After the owner approved removing
  one named empty fixture network, reinspection found it already absent and
  capacity restored. This task removed no unknown networks or storage.
- After preserving PR #2604, coverage passed on signed source
  `88bf94777710ff3444017109bc7e1474ea59b18a`: job 28692, exit zero,
  73 unit, 12 request and 500 care API tests. Complete manifest, LCOV and assessment:
  `/private/tmp/s05-i01-evidence/coverage-baseline-88bf9477/`. Raw application
  coverage is 36,654/57,626 (63.61%); API controllers remain 92.18% and OAuth
  controllers 2.91%. Removing only the three new inline test modules yields
  36,270/57,242 (63.36%) application coverage and 27,843/38,185 (72.92%) models.
  Older inline test bodies may remain counted. These figures do not establish
  operation acceptance or satisfy every coverage target.
- Initial cumulative CI job 8997 passed Rust/integration checks, then failed
  before browser tests because the stale asdf `npx` shim could not find asdf.
  Selecting the existing mise Node installation through scoped PATH made the
  unchanged browser prerequisite pass. No source or global configuration changed.
- Full `task --concurrency 1 ci` retry on the same signed source passed:
  job 68704, exit zero; 73 unit, 12 request, 486 selected care API and all 662
  desktop/mobile browser cases. Care reported one ignored and fourteen filtered;
  requests intentionally ignored the incomplete whole-API gate. Other integration
  shards and repository gates passed. Log:
  `/private/tmp/s05-i01-evidence/full-task-ci-88bf9477-path-recovery.log`.
  Separate workflow gate job 13924 passed all 137 policy checks with the scoped
  upstream actionlint fix; log `ci-check-88bf9477-final.log` in the same folder.
  Owned fixtures were cleaned and generated screenshots restored after retaining
  their patch. The unrelated Serena configuration remains preserved and excluded.
- Ownership coordination allocated rate-limit wiring, isolated modules/config
  and nine missing 429 entries to this API work. It also transferred the narrow
  missing-household error adapter correction; care/review edits remain owned by
  their existing coordinator. Three unpublished review-lease operations will
  require reconciliation when integrated; the current contract still has 119.
- Publication and exact-published-head hosted checks remain pending. Next is
  shared HTTP behaviour, beginning with failing full-router rate-limit tests.

Coverage publication and hosted review correction:

- PR #2607 was published on the harness branch at signed `b64532f9`, 423 additions
  and 34 deletions. PR #2600 was updated to signed `d8201065`, 1,115 additions
  and 21 deletions against `main` at `60e7de5d`. Both remote heads were verified;
  neither is merged or deployed. The generated title's initial validation failure
  was superseded by successful corrected-title run 38440132257.
- Hosted review comment 4261986300 identified valid custom coverage output under
  an unignored checkout directory being counted as a source mutation. A real-Git,
  fake-provider runner regression reproduced it: 25 passed, one failed. The
  correction excludes only untracked files in this run's UUID report directory.
  Tracked and unrelated untracked changes remain detectable, including siblings
  in the report parent. The initial 27 runner tests passed; logs are
  `coverage-output-fingerprint-red.log` and
  `coverage-output-fingerprint-isolated-green.log` in the evidence folder.
- Same-session review found asymmetric decoding of unusual Git filename bytes.
  A real-Git index regression reached the intended fingerprint mismatch before
  correction. The final filter preserves retained status bytes and all 28 runner
  tests pass. Evidence: `coverage-fingerprint-nonutf8-red-corrected.log` and
  `coverage-fingerprint-nonutf8-green-final.log`. An earlier macOS fixture error
  and failed provisional correction are retained but not credited as success.
- Hosted coverage on published `b64532f9` passed, with the clean merge revision
  `4a0dce7ec149b6a65c77e21d3cb04f569c880b41` in its complete three-selection
  manifest. Artifact 11835597271 is retained under
  `/private/tmp/s05-i01-evidence/hosted-combined-coverage-b64532f9/`. Generated
  client determinism and Swift/Kotlin compilation also passed on that PR run.
- The correction remains on its owning coverage branch. The next rate-limit
  branch's test-only work and progress record are preserved in a named stash.
  Same-session `living-pick` re-review accepted both corrections (job 73680,
  exit zero; `coverage-output-final-*` review packet, output and export). It
  confirmed existing instrumentation evidence remains applicable because Cargo
  commands and selections are unchanged. Formatting, documentation, workflow
  policy and whitespace checks passed.
- Signing failed twice through 1Password; the owner explicitly authorised an
  unsigned fallback. Commit `62422c280d7daf2d6539d1944b6881256f0a6ee6` uses only
  a per-command signing override. Global signing settings remain unchanged.
- Cumulative `task --concurrency 1 ci` passed, job 80817 exit zero: 73 unit,
  12 request, 486 selected care API and all 662 browser cases (18.2 minutes).
  The run began against the frozen staged patch on `b64532f9`; its SHA-256
  `9ccdad38333596068f4df89b8c3d7805a86947d277898c8b533de61b9e3ad72e` matches the
  correction subsequently committed as `62422c28`, and all three file hashes
  remained unchanged after CI. Full log:
  `/private/tmp/s05-i01-evidence/coverage-fingerprint-full-ci-b645-staged.log`.
  The 31 generated screenshots were preserved as a patch and restored. Task
  cleanup completed; no separate post-run Docker inventory was credited.
  Republication, exact-head hosted checks and propagation remain required.

Shared HTTP rate-limit work:

- The coverage correction and its final record are published at `2e84496b` on
  PR #2607, 582 additions plus 34 deletions against its harness base. The exact
  hosted review thread was answered in comment 4262482794 and verified resolved.
  Both earlier published heads passed hosted checks; new-head checks are active.
- `codex/s05-i01-http-rate-limits` is rebased onto `2e84496b`. The writer's
  preserved request tests are restored. Focused RED job 46801 compiled and ran
  all three TestServer cases; each expected 429 after exhausting an export bucket
  but received 401. Log: `/private/tmp/s05-i01-evidence/http-rate-limit-red.log`.
  The failure proves missing runtime limiting, not a fixture or compilation fault.
- The API writer owns the bounded model, middleware, view, configuration and
  nine approved 429 contract corrections with their tests. The coordinator owns
  records/publication; the verifier owns builds and fixtures. Existing care and
  review ownership remains unchanged. No operation is credited complete yet.
- Initial GREEN attempt 98002 stopped at a configuration-error adapter type
  mismatch; no tests executed. After that compile correction, policy RED job
  41810 ran eight tests: six passed and two failed on an undocumented descendant
  path receiving a special limit and a trailing empty trusted-proxy entry being
  accepted. Missing-peer RED job 97959 executed one TestServer case and failed
  because the error response was plain text instead of JSON. The writer owns
  these corrections. Logs are `http-rate-limit-policy-red-escalated.log` and
  `http-rate-limit-missing-peer-red.log` under the existing evidence directory.
- Policy GREEN job 33947 passed: eight limiter unit cases and one existing push
  retry case matched the selector. This proves the focused model corrections;
  request GREEN job 85558 then passed all four TestServer cases. The verifier
  released the writer for the Loco rendering adapter and approved OpenAPI and
  inventory edits before broader verification. Independent review and final CI
  remain required. Focused logs are `http-rate-limit-policy-green.log` and
  `http-rate-limit-request-green.log` in the evidence directory.
- Broader inventory job 7207 exposed a configuration regression: 13 passed,
  three baseline boot cases failed and one completeness test remained ignored.
  Empty unquoted proxy settings render as YAML null, rejected by the strict
  string parser. The writer owns quoting all three environment templates;
  the passing focused tests had overridden the setting with an explicit string.
  Inventory evidence will also name the exact unchanged medication-take, stock,
  audit and sync projections instead of the overly broad `no-write` label.
- Independent Devin SWE-2 Max review uses session `ambiguous-jupiter`. Initial
  job 87550 stopped at unavailable noninteractive Serena permissions without a
  verdict. Read-only review resumed as job 71657 with the source and verification
  reference supplied directly; it has no build or source-edit ownership.
- Quoted configuration and precise evidence labels passed inventory job 5736:
  16 passed, one completeness gate ignored. Focused request job 95758 passed all
  four rate cases again. Route reconciliation remains 118 of 119, with the
  documented push-subscription test route still missing. Logs are
  `http-rate-limit-inventory-green-v2.log` and
  `http-rate-limit-request-green-v2.log`; these results do not close acceptance.
- All 59 hosted checks passed on published coverage head
  `2e84496b3222c1d60595d4d783b85d1dc6b0c82c`. Coverage PR #2607 remains open and
  unmerged. The check snapshot is `coverage-head-hosted-checks-latest.json` in
  the evidence directory. Rate-limit unit job 81742 passed all nine matching
  cases (eight limiter cases); formatting job 93354 passed.
- Strict Clippy job 50528 passed across all targets with `-D warnings`. The full
  contract group job 3852 passed 16 cases and failed only its intended 119-operation
  completeness gate. These are recorded separately from full root CI, still pending.
- Devin session `ambiguous-jupiter` completed its supplied-source review as job
  24562 after read-tool permission prompts stopped the earlier attempts. It
  accepted the bounded implementation conditionally on an explicit HEAD-policy
  decision: Axum's implicit HEAD read routes may bypass the GET-specific buckets.
  The writer added a test-only authenticated HEAD export regression; verifier
  job 81844 is checking reachability before any policy or contract correction.
  The proposal is `/private/tmp/s05-i01-evidence/proposed-head-rate-policy.md`.
  Non-blocking review observations remain in the retained review output; no
  unrelated framework or authentication rewrite is included.
- HEAD regression job 81844 confirmed reachability and the bypass: authenticated
  HEAD returned 200, nine GETs from the same client returned 401, and the final
  authenticated HEAD returned 200 instead of 429. The owner has been asked to
  approve sharing GET buckets for implicit HEAD reads plus an explicit OpenAPI
  sentence. The exact unapplied diff is `proposed-head-rate-policy.patch` in the
  evidence directory. Production changes, full CI and publication are held for
  this contract decision; elapsed time is not approval.
- Pure shared GET/HEAD-bucket regression job 61734 also failed as intended:
  one case executed, 81 filtered, with no rejection where the shared read bucket
  should be exhausted. Both new HEAD regressions remain test-only changes;
  the policy and OpenAPI proposal has not been applied. No build is left running.
- The owner authorized the HEAD correction on 13 October if HEAD poses a
  denial-of-service risk. The condition is met: the authenticated request test
  proves HEAD executes the export handler while avoiding its stricter quota.
  Apply the three shared GET/HEAD read buckets and clarify the contract wording.
  The earlier pending-decision entries above are superseded. Verification remains
  held while the migration coordinator runs final dm+d/care CI and prepares its
  authorized merges; its completion message will release the shared build lane.
- Approved HEAD correction is now applied locally: all three read rules share
  GET/HEAD buckets, the OpenAPI introduction is clarified, and its test pin is
  `701b093a1fd87203769e19a393e7524bd6e9a0dc6f4c7394c20750d643e9ffa4`.
  Inventory remains 119 operations with partial export evidence and unchanged
  native pins. Static whitespace and JSONL checks pass; focused GREEN, same-session
  Devin review and full CI remain pending the shared-lane release. The complete
  local patch is `http-rate-limit-head-candidate.patch` in the evidence directory.
- On 14 October the migration coordinator released the build lane after dm+d
  PR #2615 merged. Refreshed main is `99154e36686db793952cfc0120de4d7469079320`.
  GitHub confirms prerequisite PRs #2600 and #2607 are also merged. The stack
  rebase skipped those merged layers and based this rate-limit layer on main.
  All WIP was preserved in named stash `s05-i01 approved HEAD candidate before
  merged-main refresh`; the recovery copy remains. Three configuration conflicts
  were resolved by keeping main's centralized settings and adding the quoted
  rate-limit setting. No module collision was found. Refreshed GREEN verification
  is starting; the prior shared-lane hold is superseded.
- Refreshed unit job 48584 passed ten matching cases (nine limiter cases,
  including shared HEAD/GET buckets, plus the existing push retry case).
  Initial request job 52501 stopped at compilation because upstream changed
  the identity exchange helper signature; no request evidence was credited.
  After matching existing fixture calls, request job 90471 passed 17 cases with
  one intentionally ignored acceptance gate, including authenticated HEAD export
  throttling. Route inventory stays 118/119. Formatting passes. Devin correction
  review resumed in `ambiguous-jupiter` as job 8036; full CI remains pending.
- Devin correction review job 8036 approved the refreshed candidate for full CI;
  F1 is resolved by the approved policy, model regression and authenticated request
  regression. Remaining observations are non-blocking and retained in
  `rate-limit-head-correction-output.log`; they do not establish full API acceptance.
- Frozen source `95259214276c2cad7deebcf2bea551757a5b1296` ran full CI as job
  61497 and exited 201 after normal cleanup. All 99 units and 664 browser cases
  passed; OAuth integration had 16 passes and one failure: the existing socket
  fixture's capabilities request returned 503 instead of 200 because its raw
  router server omitted ConnectInfo. Standard Loco production serving supplies
  it. The writer owns the narrow fixture adapter, with no middleware weakening.
  Docs build passed. The 33 generated screenshot diffs are preserved in
  `ci-95259214-generated-screenshots.patch` and restored; unrelated Serena changes
  remain preserved. Focused regression, correction review and final full CI are
  required before publication.
- Socket-fixture correction now matches Loco production ConnectInfo serving;
  the missing-peer rejection and expected successful capabilities response are
  preserved. Focused OAuth job 46234 passed, then full OAuth job 98800 passed
  all 17 cases. Unit job 90234 passed ten matching cases (nine limiter cases),
  inventory job 80205 passed 17 with one intentional ignore, and formatting job
  67443 passed. Initial unit job 89634 stopped at a missing test-only enum import;
  no Rust test execution was credited for that compile failure. The new tables
  cover strict configuration types and HEAD negative paths without production
  changes. Same-session Devin correction review is running as job 53420 before
  the final commit and required full CI rerun.
- Devin correction review job 53420 approved the production-equivalent socket
  fixture and typed negative cases. No blocking findings remain for this bounded
  rate-limit candidate; full root CI on its committed freeze is the remaining
  publication gate.
- Final full CI job 47196 on `56a3b1f3439e186dcb081ad2ffe4aded5ed705ac`
  exited 201 after normal Task cleanup. All 99 units and 664 browser cases passed
  (browser duration 17.1 minutes); request inventory passed 17 with one intentional
  ignore. Care API integration passed 489, failed one and ignored two, with 14
  filtered: the dm+d timing test reported `missing extract timing: []`. The
  sequential integration runner stopped there, so later OAuth tests did not run;
  separate full OAuth job 98800 remains the valid 17/17 result. The 31 generated
  screenshot diffs were preserved in `ci-56a3b1f3-generated-screenshots.patch` and
  restored, retaining unrelated Serena configuration.
- Original-source timing job 2141 passed alone (one pass, 505 filtered), but
  dm+d module job 1917 reproduced the same failure (17 passes, one failure, one
  ignore). This establishes an interaction in the broader run, without proving
  its cause. All verifier jobs have exited. A narrow diagnostic patch records a
  tracing probe, persisted import statuses and raw events; it remains unapplied
  pending permission to coordinate ownership of the dm+d test. No production
  correction, unchanged full-CI retry, rate-limit PR publication, merge or
  deployment occurred. The reviewed GET/HEAD policy and all-119 acceptance scope
  are unchanged; required CI failure blocks dependent work.
- Care/review PR #2617 is now confirmed merged at 08:38:23 UTC on 14 October as
  `672dbc0d1520ee586ce625f0dbfec7fb3eba0544`; the coordinator released its hold
  and reported local cleanup. GitHub merge identity was checked before refresh.
  The API stack rebased onto that main, retaining unsigned commits as authorised;
  current replayed head is `7880e85c9`. Progress and unrelated Serena changes were
  preserved in named stash `s05-i01 preserved progress and Serena before care2617
  refresh`, then restored while retaining the recovery stash. The sole request-test
  conflict keeps main's 122-operation count and lease metadata alongside all rate
  tests and partial evidence. Main already contains the equivalent inventory patch,
  so its earlier unapplied handoff is superseded.
- The current schema plus the approved rate declarations hashes to
  `bef3d7a3db6d51d6756c864301f24821cd6b53ed8f6dbc68196bc6f7eff1f063`.
  Static reconciliation found 122 operation rows, 121 route pins, no extra routes
  and the preexisting missing push-subscription test route. Coverage remains two
  partial and 120 unverified operations. Acceptance now includes the three merged
  lease operations; the historical 119-operation results remain separate. Main
  also binds the dm+d timing subscriber to each import future; this test-only
  correction is retained, with local regression verification still required.
  New lease responses omit the existing global 429 contract, so the next bounded
  correction starts with a failing metadata assertion before updating those
  declarations and inventory. No new rate policy or native pin update is planned.
- Initial lease metadata job 70308 stopped at test compilation (E0716); no
  behavioral result was credited. After a test-only binding correction, job
  72917 executed the assertion and failed on exactly the three lease operations
  missing shared RateLimited 429 responses. The correction adds those three
  declarations, their inventory response/schema/header entries and schema SHA
  `acfdc42aee410991c16183a65074bb1e2961397722731a88b4477b34fad831eb`.
  Inventory job 69691 then passed 18 with one deliberate ignore; limiter unit
  job 51413 passed ten matching cases. Merged dm+d module job 45508 passed 18
  with one ignore and 504 filtered, including the formerly failing timing case.
  No new timing diagnostic or production importer correction was applied.
  Evidence and coverage states remain unchanged. Independent correction review,
  current coverage measurement and full CI remain required before publication.
- Frozen unsigned candidate `bbd971dd13fda0b3450d0a5e656f2c8ccb26ac5e` passed
  formatting, strict all-target lint job 46272, docs build and diff checking.
  Same-session Devin `ambiguous-jupiter` correction job 99421 accepted it with
  no blocking findings. Report-only coverage job 6021 exited zero: 102 units,
  18 requests (one ignored) and 521 care API cases (two ignored) passed. Manifest
  `rate-coverage-main672/3332732b-b1f3-47c6-85e4-d1898b33e063/manifest.json`
  records that revision and source-state SHA
  `511e72ee1181e0f15c371dcbdec0189a87cdef24f37bc41de2b0b0f968f107b0`;
  dirty state is the preserved unrelated Serena file. Raw LCOV is retained there.
  Excluding new inline test bodies/helpers, limiter model coverage was 161/171
  (94.15%), controller 46/46 and view 16/16. Model gaps were error-display text,
  malformed settings rejection and stopping a forwarded chain at the first
  untrusted hop. API controllers aggregate 92.33%; raw source aggregate 63.20%
  and models 73.17% include inline test bodies. OAuth controller coverage is
  2.88% because this selection omits its separate test binary. These are baseline
  limits, not operation acceptance or proof that all business paths pass.
- Two test-only additions now exercise actual quota sharing across valid and
  malformed spoofed forwarding prefixes, and five invalid configurations with
  their operator-visible startup errors. Production code is unchanged. Focused
  unit job 85432 passed 12 matching cases (11 limiter cases plus push retry).
  Formatting passes after correcting the new test layout; no behavioral result
  was inferred from the formatting failures. Same-session correction review,
  coverage remeasurement and full CI remain before publication.

- Final runtime candidate `e124b30da85cab62445fd94eac42bb94eb3f7f3e` passed
  `task --concurrency 1 ci`, job 37169, exit zero: 104 units, 507 care API cases
  (two ignored, fourteen filtered), 17 OAuth cases, 18 request cases (one ignored)
  and all 744 desktop/mobile browser cases. Other integration binaries passed.
  Strict all-target lint job 88178, formatting, docs and whitespace also pass.
  The full log is `rate-limit-full-ci-e124.log` in the evidence directory.
  The Task exited normally. Its 40 regenerated screenshots were preserved in
  `ci-e124-generated-screenshots.patch` and restored; unrelated Serena work remains.
- Coverage job 72508 exited zero on the same runtime candidate: 104 units,
  18 requests (one ignored) and 521 care API cases (two ignored) passed. Manifest
  `rate-coverage-e124/a192519b-ef8b-416c-8d55-14e88ce6d7cd/manifest.json` records
  source-state SHA `4368f2d688c442f4c2b8251446b2d1db1d5627532a4cdfd91c93a0b35d744158`.
  Runtime line coverage is 171/171 for the limiter model, 46/46 for its controller
  and 16/16 for its view, excluding inline tests and test helpers. Broader source
  coverage remains a report-only baseline; this is not complete API acceptance.
- Same-session Devin `ambiguous-jupiter`, SWE-2 Max correction job 37262, accepted
  the final test additions without blocking findings after job 99421 accepted
  the merged-care reconciliation. The coordinator owns publication and evidence;
  `/root/api_writer` owns code/tests and `/root/verifier` owns verification and
  resources. The reviewed runtime diff is 1,354 lines against its immediate base.
  The following documentation update records results without changing runtime.
- Hosted PR #2623 review raised capacity retry timing, HEAD bodies and a proxy-trust
  warning. Same-session adjudication job 55410 confirmed the capacity defect:
  unit RED 53145 returned retry 20/reset 1020 instead of 200/1200. The correction
  uses the earliest retained expiry only when capacity prevents a new counter.
  Mock-transport full-router HEAD tests passed unchanged for 503 (85332) and 429
  (36073); Axum 0.8.9 strips the body after middleware, so no duplicate handling
  was added. The proxy warning was a deployment documentation gap: configured
  proxies must overwrite forwarding values or append their observed client peer.
  A trusted-looking spoofed prefix now also passes the existing protection test.
  The description-only OpenAPI update has SHA
  `fae2da8212338c04dea75628e35c5892e5e5bbde722d160b3072048c6ea31036`;
  operation metadata and native pins are unchanged. Unit GREEN 9441 passed 13;
  request GREEN 2433 passed seven. Formatting, strict lint 60133, inventory 74148
  (20 passed, one ignored) and whitespace pass. Final correction review, coverage
  remeasurement and full CI remain required before publishing the updated head.

Session close-out on 11 October 2026:

- [Profile PR #2504](https://github.com/damacus/med-tracker/pull/2504) is merged
  as `b50951e09323060ec7a8d6c6f08a0750db0c3b25`.
  [Drawer/account-menu PR #2521](https://github.com/damacus/med-tracker/pull/2521)
  is merged as `a1e18b0172c398c18c33cf967e7857104692aa52`. Profile settings use
  accordions showing current values; navigation belongs in the drawer and
  appearance settings live on the profile page. Parent CI issue #2522 is closed.
- [Dashboard PR #2505](https://github.com/damacus/med-tracker/pull/2505) remains
  open at signed commit `ab0ad668acc8586443d8c6557738c1022dc28873`.
  Review fixes preserve account-local schedule dates, taper previews and recorded
  doses, and avoid repeated stock resolution. The dashboard reuses nine existing
  profile/navigation SVG symbols instead of duplicating them.
- Local `task ci` passed, including all 586 desktop/mobile browser cases.
  Fresh [hosted run 38121077030](https://github.com/damacus/med-tracker/actions/runs/38121077030)
  passed all Rails unit and system shards on `ab0ad668`. At the close-out snapshot,
  Loco Foundation was still running; all other reported checks passed. This does
  not establish complete hosted acceptance, merge or deployment.
- The earlier Rails scanned-stock failure passed on hosted rerun. Intermittent
  notice, menu and lookup assertions remain tracked in
  [#2526](https://github.com/damacus/med-tracker/issues/2526). No Rails assertions
  were weakened and no automatic retries were added.
- A screenshot alleged a dashboard Cedar view-access bypass. Current grant and
  membership checks were inspected; eight dashboard tests and the Cedar
  grant/expiry test passed. No bypass was reproduced. No security finding was
  dismissed, and no corresponding unresolved GitHub review thread was present.
- [Profile acceptance #2502](https://github.com/damacus/med-tracker/issues/2502)
  and [archive restoration #2513](https://github.com/damacus/med-tracker/issues/2513)
  remain open. Gravatar remains disabled. Production cutover is not ready and
  no deployment is authorised by this close-out.

Next: inspect the final Loco result for `ab0ad668`, address any failure on that
revision, then obtain dashboard acceptance. The shared HTML report remains at
`/private/tmp/medtracker-migration-status.html`; its authoritative structured
input is `/private/tmp/medtracker-progress-report.md`. This ledger update is
published directly to `main` at the owner's request, separately from PR #2505.
Other sections below retain their historical evidence dates.

## Earlier delivery checkpoint — 6 October

Continuation [PR #2460](https://github.com/damacus/med-tracker/pull/2460) publishes
the Rails shared-ignore correction at signed Dan Webb commit
`730942fd2680439d77f626f5c7fef24fa4836c70`. Exact-head hosted
[CI 37504518956](https://github.com/damacus/med-tracker/actions/runs/37504518956),
GitGuardian and title validation pass. Local Rails baseline passed 6,112 examples
with no failures. The separate release PR #2381 proposes version 0.6.0; it is
not merged or deployed.

The current care sync and dose-record browser candidate passes final root
`task ci` run 85878: 141 care API tests, all remaining Rust targets and all
192 desktop/mobile browser tests. The browser run completed in 11.6 minutes
without skipped or flaky cases. Persistence includes its one explicitly ignored
schema-capture utility. Task-owned test resources were cleaned up.

Devin SWE-2 Max cleared the occurrence browser changes and all corrected sync
operation families. The final location regression first reproduced permission
checks occurring after missing/stale version checks; all 24 focused sync cases
now pass with authority checked first. Snapshot visibility, deletion delivery,
replay permissions and exact response behaviour are covered by the final run.
These care changes await publication and exact-head hosted acceptance.

Identity integration awaits the owner's durable Better Auth patch-source choice.
Reminder workers compile and pass six controlled delivery tests. Reviewed retry,
partial-delivery and scheduling gaps are being corrected within this delivery.
Production reminder lifetime and uncertain sends after a crash still await owner
decisions. PDF downloads and existing report information are being implemented;
their first compilation follows care verification. Scratch configuration passes
four tests and its orchestration passes five; actual images remain unproved.
Saved-state rollback preparation passes eight orchestration tests; populated
restore and Rails operation remain unproved.
Remaining operations, integrations, browser journeys, workers, both scratch
architectures and rollback remain required. The authoritative current report is
`/private/tmp/medtracker-migration-status.html`.

## Approved cutover scope — 6 October

Rollback restores an independent pre-cutover dump or frozen replica and restarts
Rails; new Loco data may be lost. Prove restoration and Rails operation, rather than
Rails compatibility with Loco-created data. Stop Rails servers, workers and schedulers
before Loco starts; stop Loco writers before restoring Rails. Old pending/failed Rails
jobs may be lost without queue transfer or individual delivery reconciliation.

Unsupported historical passkeys may be invalidated without counting affected accounts
as a gate. Invalidate existing sessions/tokens with a proved mechanism, preserving
fresh sign-in and MFA. Historical links, export formats, offline queues and push
subscriptions need not survive. Preserve records/files, new authorised downloads,
new exports, future offline replay and future push behaviour.

Use daisyUI, the same colour schemes and clear Loco browser routes. Exact Rails pixels,
CSS geometry, theme-export fidelity and browser bookmarks are excluded. Required
information/actions and accessibility remain. PDF reports must match existing appearance
and work in scratch. Inspect renderer/font and retained Rust work before choosing.

FHIR/SMART, MCP, AI and external medication lookup are deferred from the first
production gate, with no exact external provider/configuration parity requirement.
They remain in the full migration goal. Core medication entry/care remain mandatory.
Report first production readiness and complete migration acceptance separately.
None of these design decisions authorises live data access, invalidation or deployment.

The subsequent first-release answers keep NHS dm+d import/reconciliation/scanner,
medication reviews/background refresh, complete administration, time-limited support,
household export/closure/retention holds/deletion, avatar uploads, complete devices/
sessions and automatic live dose/stock updates in the core production gate. Offline
capture/replay, portable imports and both-native-app acceptance are deferred from
that gate but remain in the full goal. Optional AI/provider deferral cannot exclude
catalogue/scanner or review outcomes.

## Published

[Baseline PR #2451](https://github.com/damacus/med-tracker/pull/2451) was squash
merged on 6 October 2026 as
`3f4a7f952ef857b0f4f6e26105d9c39747771b0a`. Its exact source head
`2d95d11aeff02a42251b802d11f09080d54ca536` passed
[CI 37492218152, attempt 2](https://github.com/damacus/med-tracker/actions/runs/37492218152),
GitGuardian, title validation and advisory Clippy. The Loco job passed 117 care API
and 178 browser cases. The unchanged failed Rails schedule-card file passed locally
and in its single hosted shard retry; no application change was needed.

Loco owns root commands, routes, configuration and templates. Rails is independently
runnable under `rails/` as the reference and rollback application. Published
work includes account setup/invitations, supported authentication, stored API
credentials, medication/dosage/location/people management, household administration,
treatments, recorded pauses, dose outcomes and OAuth consent/token/revocation.

All eleven approved GitGuardian classifications are verified IGNORED: ten synthetic
fixtures use `test_credential`, and incident 37907524 uses `false_positive`
for fixed audit labels. No scanner exclusion or history rewrite was used.

## Published behaviour and evidence

Published Recovery passes all 14
desktop/mobile cases, including exhausted OTP, passkey-only recovery, one-use
codes, current-account checks and bound OAuth continuation.

People passes eight API cases and its desktop/mobile browser journey. Creation
preserves capacity, carer/grant/Home associations, permissions and audit/sync
effects. Partial updates retain the contract without mandatory `If-Match`.
An actual Rails service probe supplied the exact saved POST/PATCH retry digests,
including filtered email; both replay correctly. New Rust keys still bind the
submitted body. Forced audit failure rolls back every dependent write. A reproduced
lost permission-version increment is corrected with an atomic stored-value
increment. Extra request fields are filtered as Rails does, protected account IDs
cannot be assigned, and an empty filtered person returns 400.

Devin SWE-2 Max approved both Recovery and People corrections. Final combined
`task ci` passed all Rust/static/lifecycle checks, 31 care API cases and all 74
desktop/mobile browser cases. Owned test containers, volumes and networks are
absent after cleanup. The pre-correction partial run is superseded by this result.
The corrected delivery is committed and pushed. Household administration now passes
eleven API cases and six desktop/mobile journeys. Devin SWE-2 Max accepted the final
requirements, correctness and security review with no outstanding defects.
Administration is published with the passkey delivery below.

Native passkey registration, passwordless and second-factor sign-in, restart and
password-confirmed removal pass. Supported ES256 and RSA-2048 RS256 credentials
generated by Rails WebAuthn 3.4.3 sign in through the native browser without
re-enrolment, advance their retained counters and survive a server restart. Import
does not create a registration audit. The maintained library verifies the ceremony;
there is no custom signature verifier. A real concurrent removal reproduced an
account/key/session foreign-key lock-order failure; consistent account-before-key
locking now produces a clean rejection without issuing a session.

The user approved dropping unsupported historical passkey algorithms on 6 October.
This intentional compatibility break is recorded in [the deployment guide](../../deployment.md#approved-cutover-decision-unsupported-passkeys).
Supported keys work on mixed-key accounts. Security settings identify keys needing
replacement, and unsupported-only accounts receive recovery instructions after
password verification. Recovery, supported-key registration and explicit old-key
removal pass without bypassing MFA. Unsupported rows remain for Rails rollback
until authenticated removal. All fourteen focused transition, import, concurrency
and CSRF/ownership cases pass across both viewports. Devin SWE-2 Max accepted the
final requirements and correctness/security review with no outstanding defects.
Final combined `task ci` passed all Rust/static/lifecycle checks, all 41 care API
cases, three unit cases including exact challenge expiry, and all 124 desktop/mobile
browser cases on the frozen candidate. Documentation verification, commit and push
are complete for administration and passkeys. This does not accept
the remaining identity, care, browser or release requirements.

## Current implementation

Invitations and account setup are next. A real signed-in browser reached household
administration and failed at the absent Invitations link. Existing API tests also
proved the missing invitation routes. Nine invitation API tests and four persisted
credential tests pass. Desktop/mobile invitation management passes with real Loco
worker email delivery, resend and cancellation through an isolated Mailpit inbox.
Guest password and OTP sign-in resume the invitation. Acceptance committed the
expected effects, but a stale synthetic fixture ID sequence changed the selected
account person. After correcting the fixture, all six invitation journeys pass
on desktop/mobile, including password/OTP acceptance, atomic effects and replay.
Stored sessions and app tokens now pass real care-route checks, including source
household binding and immediate permission-version withdrawal. Four API-session
and five resource-scope regressions also pass. Account creation with actual email
verification remains required in this delivery. The missing POST handler is now
registered. A valid submission initially returned 503 because its conflict rule
did not match the preserved partial email index. The corrected insert passes the
representative account creation, real email delivery and verification journey.
All ten desktop/mobile signup cases now pass, including invalid, expired, revoked,
cancelled and existing-account rejection. All ten open-registration desktop/mobile
cases also pass: first-household bootstrap and verification, environment overrides
and closed-registration denial. A database permission test proves that owner
detection exposes only a boolean and preserves the application's tenant row
visibility. Final verification resend passed both desktop/mobile cases with real
SMTP delivery, strict recent-email suppression, retained key reuse and verified/
missing-account refusal. Completed queue jobs and verification-key rows remain
stored, matching the pinned framework and Rodauth lifecycles; tests count those
rows while proving that rejected requests add no effects.
Eight dosage API/removal cases pass, covering protected inactive references,
live manager revocation and audit rollback. Both desktop/mobile dosage browser
journeys pass actual create/edit/delete and parent-stock changes. Deleting the
last tracked option resets stock to untracked, matching Rails. Medication
list/detail reads now pass both route tests after two actual missing-GET cases
failed, including conditional reads, stock projections, visibility and auditing.
Valid OAuth credentials do not substitute for the invitation API's
required user session. These changes remain local and have not passed acceptance.

## Whole-migration acceptance

| Area | Implemented evidence | Still required |
| --- | --- | --- |
| Foundation | Accepted: both runners, audit, corrective review, local/docs and exact-head hosted CI pass; record published | Complete; product and populated rollback gates belong to later areas |
| Persistence | Guarded schema adoption, restricted roles/RLS, persistence and tenant tests, normal seed CLI | Populated Rails rollback and complete schema acceptance |
| Identity | Password/session and bearer tests; OAuth library integration; published TOTP, Recovery and supported passkeys | Account creation/verification, remaining security settings and complete client interoperability |
| Care | Dose, stock, medication, location and People management; published household settings, membership and care-access administration | Invitations, remaining household operations, dosage, treatment and sync |
| APIs | Actual Loco listener checks for migrated care and OAuth routes | Complete native/FHIR/SMART/MCP contracts |
| Browser | Tera/daisyUI themes and migrated care/OAuth journeys on desktop/mobile | Remaining workflows, accessibility evidence and PWA/offline |
| Workers | Owner-provisioned queue and restricted queue startup/recovery configuration | Every real job, recurrence, reminder, retry/idempotency and crash/drain outcome |
| Release | Retained source/PR inputs and rollback application preserved | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Working rules and reports

Use [the current working rules](team-charter.md). Historical relocation/workspace
and source-preservation audits run explicitly through `task migration:audit`.
Do not regenerate historical ledgers for each delivery. Duplicate runners, unused
demos and the proposed temporary worker probe were removed. Keep real database
isolation, process cleanup and side-effect verification.
The current runner has no duplicate replacement to remove. One representative
browser test now establishes a shared missing entry point; all distinct behaviour
and security cases still run after implementation. Retire helpers when their last
caller is replaced, within the same delivery.
The remaining plans use named model methods and existing tenant transactions.
The proposed command dispatcher, execution wrapper and parallel household test
binary have been removed from the plan. The existing browser command accepts a
standard test filter for prerequisite checks; it remains the full-suite runner.

Migration defects stay in this work. Create follow-up issues only for independent
scope. Useful retained PR inputs are recorded in [pr-inputs.md](pr-inputs.md);
close superseded work only after its replacement is published. Incident #2453
remains separate. Original Rails tests remain the rollback reference; Loco work
does not trigger unrelated RSpec repair.

Update the local reports at `/private/tmp/medtracker-migration-status.html` and
`/private/tmp/medtracker-2450-review-status.md` at each completed slice or material
blocker. Detailed logs stay outside the status page.
Prior delivery history is preserved in Git and the existing evidence artifacts.
Last recorded retrospective: 18:24 UTC on 6 October. Next: 20:24 UTC.
