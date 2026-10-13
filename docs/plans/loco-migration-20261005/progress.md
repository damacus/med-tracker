# Migration progress

Updated 13 October 2026. Complete every slice in [the plan](plan.md).
Foundation is accepted. The verified baseline PR #2451 is merged; all remaining
migration areas stay in scope. No live deployment has occurred.

## Current delivery

### s05-i01 — API contract completion, 13 October 2026

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
  Logs are under `/private/tmp/s05-i01-evidence/`. Coverage is pending.
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

Coverage extension on `codex/s05-i01-coverage-baseline`:

- Base: PR #2600 at `2808f38ce508f2606b6d2d0787301961bffd85d7`, including
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
- Existing hosted unit coverage at `2808f38` measured 1,931 of 57,239 application
  lines (3.37%), including zero API-controller lines. This `--lib` baseline is
  preserved under `/private/tmp/s05-i01-evidence/hosted-unit-coverage-2808f38/`.
  It does not measure the full suite. Integration measurement, cumulative CI,
  publication and affected-code coverage assessment remain pending.

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
