# Typed internal handoff stack

## Outcome and boundary

Remove the remaining internal JSON command and report handoffs in the five areas
below. Preserve browser behaviour, API/MCP formats, validation precedence, live
authority, RLS, replay, stock atomicity, audit and outbox effects. JSON remains at
transport, persisted document, configuration and final template boundaries.
No deployment is included.

Delivery uses a linear same-repository stack rooted on `main`. Existing dependent
creation PR #2896 is the precursor. Each new PR removes its whole named area and
includes its callers and regression evidence. Shared sync callers move with their
owning domain operation; the sync layer removes the remaining adapters.

| Layer | Area and branch | Parent | Acceptance and review question |
| --- | --- | --- | --- |
| 1 | Person editing: `codex/typed-person-editing` | #2896 | Browser, API, sync and profile pass typed patches; omitted/null fields, error order, permissions and audit remain equivalent. |
| 2 | Medication, stock and orders: `codex/typed-medication-writes` | Layer 1 | CRUD, dosage options, wizard, stock check and order callers pass typed inputs; replay, ETags and stock effects remain equivalent. |
| 3 | Locations, invitations and settings: `codex/typed-household-writes` | Layer 2 | All browser/API/sync write callers use typed inputs; membership, outbox, authority and idempotency remain equivalent. |
| 4 | Sync adapters: `codex/typed-sync-adapters` | Layer 3 | Pause, occurrence and health-event operations consume typed inputs without manufactured JSON envelopes; conflict and replay behaviour remains equivalent. |
| 5 | Reports and error conversions: `codex/typed-report-handoffs` | Layer 4 | Typed report rows feed insights and localisation; error consumers avoid JSON roundtrips; report scope, dates, decimals and error contracts remain equivalent. |

All intermediate layers must compile and remain safe to land. Each receives a
meaningful failing test before executable changes, focused acceptance, independent
review before publication and `task ci`. Browser evidence uses synthetic fixtures
at desktop and mobile sizes. Final acceptance audits every affected caller across
the cumulative stack and checks that no internal handoffs remain in these areas.

## Delivery controls

Do not merge any stack layer until the final PR has been published. GitHub's
native stacks currently do not support auto-merge. Use the owner's approval to
run the normal stack merge once every layer has passed its required checks,
without bypassing them. The
active `main` rules require `CI Success` and `Validate PR title`; repository
auto-merge and squash merges are enabled. The CI pull-request event has no base
branch filter. Preserve other worktrees and unrelated branches.

The root agent owns implementation, integration, heavy verification and
publication. Discovery and routine reviewers remain read-only. Implementation
ownership may transfer between layers with the exact branch and work recorded;
only one writer and one heavy verification owner may operate at a time. Ordinary
Sol routing applies to this behaviour-preserving refactor; re-evaluate if a
security decision or unresolved authority change appears.

## Evidence

- Precursor: #2896, head `aa52085e08dd56da234ef4b014960a20cd6bbb0d`.
- Person-editing interface regression observed failing before executable edits:
  `/private/tmp/medtracker-person-edit-red.log`.
- Person-editing unchanged-source browser reference passed. Its typed desktop
  and mobile screenshots are byte-identical. Full `task ci` passed: 159 Rust
  unit tests, 42 browser journeys, 1,175 integration tests and the separate
  competing-process fixture test. Log: `/private/tmp/medtracker-person-edit-ci.log`.
- Independent person-editing review found no actionable issue in the frozen
  source, caller and regression-test diff.
- Medication/stock/order typed-command interface regression failed before
  executable edits: `rtk task check` reported missing `MedicationCreate`,
  `LocationId` and typed create entrypoint. No transport or schema change was
  required. CRUD, dosage, wizard, stock check, order and sync write callers now
  pass typed inputs; writes return models until final response serialization.
- Layer 2 focused checks passed locally: `care_medications` 31/31, dosage API
  14/14, stock-order API 5/5, sync catalogue 6/6, stock-check persisted
  aggregate/audit case 1/1, wizard extra-option unit 1/1, desktop wizard and
  dose edit 1/1, and mobile stock-match/refill 1/1. The all-target `task check`,
  `task lint`, `task fmt` and `git diff --check` passed. The delivery controller's
  frozen-source `task ci` passed: 160 unit tests, 42 browser journeys, 1,178
  integration tests and the separate competing-process fixture test. Log:
  `/private/tmp/medtracker-medication-ci.log`.
- Mature unchanged-source and final browser captures match byte for byte after
  waiting for `body[data-navigation-ready]`. The [paired desktop and mobile
  evidence](../screenshots/typed-medication-writes/README.md) records source,
  viewport, locale and SHA-256 values. Earlier captures showed incomplete
  navigation and are excluded from the comparison.
- Independent reviews of medication CRUD, dosage savepoint and the remaining
  typed callers found no actionable issue; the capture guard is verified by
  the paired mature browser runs.
- Layer 3's typed-location patch interface regression failed before executable
  edits: `rtk task check` reported the missing `LocationInput` and
  `LocationField` types. Its focused unit test later passed. Location,
  invitation, household settings and membership writes now receive typed
  inputs from their browser, API and sync callers. Invitation email validation
  has a typed producer with the existing `OperationError` adapter for callers
  outside this layer; signup's JSON error consumer remains for layer 5. The
  unused grant argument was removed without changing its request validation.
- Layer 3 focused local acceptance passed: locations 26/26, invitations 61/61,
  household administration 41/41, sync catalogue 6/6, and request-level
  settings 13/13. The test-first location unit case and 19 frontend unit cases
  passed. All-target `task check`, `task lint`, `task fmt` and `git diff --check`
  passed. The unchanged-source and candidate full browser runs each passed
  42 journeys with one skipped test. [Paired mature desktop and mobile
  captures](../screenshots/typed-household-writes/README.md) show six
  byte-identical location, settings and members views; invitation management
  keeps the same layout and content apart from fixture timestamps.
- Independent whole-area review and narrow follow-up review found no actionable
  issue in the layer 3 source and test diff. The delivery controller's frozen
  `task ci` passed: 161 unit tests, 42 browser journeys, 1,179 integration tests
  and the separate competing-process fixture test. Log:
  `/private/tmp/medtracker-household-ci.log`.
- Layer 4's typed pause-command interface failed `task check` before executable
  edits (`PauseCreate` was missing). A dual-invalid pause reason/note test then
  failed against an intermediate candidate and passed after restoring the
  reason-first error. Pause, legacy reorder, occurrence and health-event writes
  now receive typed inputs from API, browser and sync adapters; returned row
  identity stays typed until the final wire projection. Raw replay bodies,
  signed occurrence keys and final transport JSON remain at their boundaries.
  Unchanged-source desktop/mobile saved-state browser references are retained
  [with candidate capture points](../screenshots/typed-care-operations/README.md).
- Layer 4 focused acceptance passed locally: pause and legacy routes 9/9
  (Nextest run `cb580d45-82ae-434b-bbb2-f43ca1550c23`; Task wrapper exit 255
  came after the passing test and cleanup because of a shell status expression),
  dose outcomes 15/15, health events 10/10, and sync occurrence reopen 1/1.
  The latter three Task runs exited 0 with logs at
  `/private/tmp/layer4-occurrence-integration.log`,
  `/private/tmp/layer4-health-integration.log` and
  `/private/tmp/layer4-sync-occurrence.log`. The final typed sync pause identity
  case passed 1/1 with Task exit 0
  (`/private/tmp/layer4-sync-pause-final.log`). The final browser Task passed
  3/3 journeys (`/private/tmp/layer4-browser-final.log`): eight screenshot
  pairs are byte-identical, two desktop pairs have 28 one-channel-level pixel
  differences confined to the sidebar search control, and the not-taken pair
  differs only in the recorded time. Independent affected-family and narrow
  follow-up reviews found no actionable issue. The final all-target `task check`,
  strict `task lint`, `task fmt`, `task docs:build` and `git diff --check` passed.
  Root-owned `task ci` passed: 163 unit tests, 1,181 integration tests, the
  separate competing-process fixture check and 45 browser journeys. Log:
  `/private/tmp/medtracker-sync-ci-owned.log`. Docker's address pools were full
  of other worktrees' networks, so the successful run used the existing owned
  database/storage mode on a freshly created synthetic fixture stack, which
  schedules integration then browser tests. No application or Docker setting
  changed to recover the gate. Layer 5 remains planned.
- Layer 5's typed report interface and minute-projection regression failed
  before executable changes (`OrdinaryHistoryReport` and
  `within_ninety_report_minutes` were missing). Report models now keep GP,
  ordinary and medication-review rows typed through insights, dashboard,
  browser and API callers and PDF localisation. SeaORM subqueries restrict GP
  takes to the selected person in the database; selected records receive
  batched source, medicine and location enrichment. Signup email and invitation
  failures use typed errors before the existing external response mapping.
  Focused local acceptance passed: report HTTP/PDF 19/19, signup 10/10,
  PDF-localisation unit 5/5, typed-report unit 2/2 and reports browser 7/7.
  [Paired reports screenshots](../screenshots/typed-report-handoffs/README.md)
  at desktop, 390px and 320px are byte-identical to the unchanged-source
  reference. Independent signup, GP and ordinary/PDF/controller reviews found
  no actionable issue. Final all-target `task check`, strict `task lint`,
  `task fmt`, `task docs:build` and `git diff --check` passed. The delivery
  controller's final `task ci` passed: 165 unit tests, 1,183 integration tests,
  the separate competing-process fixture check and 45 browser journeys. Log:
  `/private/tmp/medtracker-reports-ci-fresh.log`. The first full run stopped
  because the retained synthetic OAuth template contained an expired code;
  its failed clone's code expired at 12:06 UTC before the test ran at 12:44 UTC.
  The focused failure reproduced, then all 24 OAuth compatibility tests passed
  after rebuilding only that task-owned template. The full rerun created the
  template fresh at first use and passed without an application change. Logs:
  `/private/tmp/medtracker-reports-ci.log`,
  `/private/tmp/layer5-oauth-expired-template-red.log` and
  `/private/tmp/layer5-oauth-fresh-template-green.log`.
- Before final-layer publication, person editing was published as #2898,
  medication/stock/order writes as #2900, household writes as #2901 and sync
  adapters as #2902. All four had passed hosted checks and remained open in
  native stack #2899 for the final-layer gate.
- The final scope audit also tracks two separate areas outside this stack:
  app-token issuance and browser review-prompt editing. Browser occurrence forms
  belong to layer 4; report reads and review-prompt PDF conversions belong to
  layer 5. Completion of these five areas does not mean every application use
  of JSON has been removed.
