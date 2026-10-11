# Care UI parity subplans

Owner-requested follow-ups, 7 October 2026. Parent: [browser delivery](06-browser.md).
Profile has its own [item-by-item subplan](06-profile-parity.md).

## Shared delivery contract

Reuse the established Rails information architecture and interactions in the
current Loco themes. Inspect rendered reference states before implementation;
the initial findings below are source comparisons, not completed visual audits.
Inventory every visible item, conditional state and reachable action per journey.
Keep meaningful existing designs rather than substituting generic inline forms.
Existing exclusions and auth ownership remain; record conflicts explicitly rather
than silently dropping a reference item or reviving deferred external providers.

For each subplan: write failing observable browser tests, implement working
operations and presentation, find and fix accessibility/localization defects,
capture desktop/mobile evidence, obtain independent review, run required checks
and publish a PR. Use one implementation writer and one costly verification lane.
Keep live-update draft preservation and one-second foreground behaviour intact.
Do not merge, deploy, mutate live records or claim parity from placeholders.

## C1: Person overview and medication cards

Reference: `rails/app/components/people/show_view.rb`,
`rails/app/components/schedules/card.rb` and its child components,
`rails/app/components/person_medications/card.rb` and its child components.
Current Loco: `assets/views/people/show.html`, `assets/views/treatments/index.html`.
The candidate largely lists medication names and separates treatment management.

- [ ] Inventory identity/avatar, overview facts, quick actions, medication source
  types, empty states, permissions and responsive placement.
- [ ] Port informative medication cards: dose/status, readable timing and limits,
  pause history and appropriately prioritized actions. Share timing-summary rules
  with dose-option and treatment editing; distinguish plans from maximum limits.
- [ ] Retain selected-person context into add/edit/record flows, authorization,
  data freshness and draft-safe live updates.
- [ ] Verify populated/empty/paused/due/view-only states and action effects on
  desktop/mobile; show before/after reference captures and reviewed implementation.

Current C1 dependency findings: root Loco has no health-events browser page or
direct CRUD API in this branch; sync-domain support alone is not a user journey.
Household-manager carer administration exists, but does not cover Rails'
person-specific parent/carer management for every authorized dependent carer.
Complete these missing operations/journeys within the approved delivery rather
than adding dead links or broadening permission. Protected local avatar reads
exist only in the separate profile draft; use a local initials fallback until
that route is integrated. These items remain open after the first card-layout PR.

## C2: Medication details and stock interactions

Reference: `rails/app/components/medications/show_view.rb`, SupplyStatusCard,
WarningsComponent, StandardDosageComponent, DoseHistoryComponent, RefillModal,
AdjustInventoryModal and AdministrationModal.
Current Loco: `assets/views/medications/show.html`, `stock.html`, `order.html`.

- [ ] Inventory header/location, description, warnings, dosage, supply status,
  order details, dose history and their conditional visibility.
- [ ] Restore information hierarchy and supply presentation within current themes.
  Keep routine recording/refill prominent and infrequent management secondary.
- [ ] Restore focused refill, adjustment and dose-administration interactions,
  preserving underlying operations, validation, duplicate protection and audit.
- [ ] Apply the 8 October approved Record dose review: prominent person, medication
  and dose confirmation; theme-token surfaces and primary action; grouped compact
  actions; honest single-source stock summary; clear repeated-form names; precise
  field errors and focus recovery. Verify keyboard use, reflow, target sizes and
  contrast across the selectable light/dark themes, with representative screenshots.
  Reuse existing translated dialog labels and mark their language; surrounding
  page localisation remains part of the wider slice 06 acceptance.
- [ ] Preserve informative order/receipt state and relevant history. Record any
  legacy external-guidance section against the approved provider scope explicitly.
- [ ] Verify keyboard/focus/close behaviour, stale edits, low/untracked/available
  stock, warnings, errors and permission denial; prove persisted quantities and
  exactly-once effects, not only modal visibility.

Implementation discovery: root Loco has absolute stock adjustment and ordering,
but no additive refill operation. Rails `RestockMedicationService` validates a
positive quantity and restock date; `Medication#restock!` adds delivered stock,
sets the last-restock baseline and clears reorder state. C2 includes the missing
domain/browser operation and its authorization, audit, stale-write and duplicate
submission checks. An order form labelled "Refill" does not satisfy this journey.

## C3: Inventory search, cards and scanner

Reference: `rails/app/components/medications/index_view.rb`, SearchComponent,
ListItemComponent, InventoryScanModal, `rails/app/components/barcode_scanner.rb`,
and barcode_scanner/inventory_scan JavaScript controllers.
Current Loco: `assets/views/medications/index.html` is a name-and-stock list;
root browser routes have no scanner/finder journey at this audit checkpoint.

- [ ] Inventory and port reference search/filter controls, informative card grid,
  empty/no-results states, stock/location indicators and permitted actions.
- [ ] Port camera start/stop, decoded result handling and manual barcode entry.
  Cover denied permission, unavailable camera, invalid input and retry feedback;
  stop camera resources when leaving or closing the scanner.
- [ ] Implement tenant-authorized local barcode/GTIN resolution, useful match
  selection and no-match/manual-entry continuation. Do not expose barcode as a
  routine medication form field merely because scanning stores it as metadata.
- [ ] Complete scan-to-restock with medication/location/current supply preview,
  quantity entry, confirmation, persisted stock and duplicate-submission safety.
- [ ] Complete scan-to-medication-entry using imported NHS dm+d catalogue data.
  Existing Rails/retained Rust lookup code is prior art, not a working Loco route.
  Missing import/reconciliation and worker work remain within approved delivery;
  do not resume an unrelated paused worker checkout without its owner contract.
- [ ] Prove imported-catalogue hit, existing-stock hit, miss, invalid/ambiguous
  match, unauthorized match and failure/retry. Use deterministic camera decoding
  in browser tests and separately record real-device camera evidence or its gap.

The scanner is absent from root Loco, not absent from Rails. A camera widget alone
is not completion; catalogue resolution and successful medication/stock outcomes
are required. Optional external lookup providers remain deferred.

### Scanner implementation record, 10 October 2026

The owner selected the complete scanner journey, including catalogue imports and
the staged Add Medication wizard. Ambiguous matches must show medicine/location
choices; no first-match or silent stock merge is permitted. For dosage-option
inventory, show strength, unit and current supply and add the confirmed quantity
to the selected option with its aggregate update in the same transaction.

Retain local NHS, imported CSV, cached product and curated catalogue sources in
their configured order. Network lookup providers remain deferred. Both browser
lookup and the existing public medication_lookup contract use the shared resolver;
the public optional existing_medication is present only for a unique match.

Initial wizard submission atomically saves the medication, submitted dosage
options, stock and authorised person plan. Its post-commit dosage summary offers
Manage dose options and Done; subsequent edits are independent transactions.
Ambiguity discovered on submission retains the draft and writes nothing until an
explicit choice is confirmed. Full-page, modal and slide-over variants remain.

NHS release upload uses private RustFS storage and the existing PostgreSQL queue,
with automatic processing and five-minute stale-import reconciliation. No new
external release-download schedule is introduced. Preserve archive verification,
safe extraction, progress, single active import, interruption and terminal cleanup.

Astra owns the tightly coupled source and tests; the coordinator owns reference
runtime evidence, integration, independent review and publication. Astra is retained
for implementation because stock transactions, public contracts and imports that survive restarts
share consequential boundaries. Focused behavioural RED/GREEN checks precede the
final frozen-candidate CI and independent code/security and three UI reviews.
Rails baseline 949535c3e is captured from isolated port 51702 with synthetic owner
admin@example.com and platform-admin access for the import pages, English,
account timezone UTC (browser Europe/London), system appearance, 1440×1000 and
390×844. Images are under
`docs/screenshots/scanner-migration/rails-*`. The reference scanner added 7 to 28
and persisted 35; the wizard committed 1 tablet daily with 28 supply. The initial
import failure `archive_persistence_failed` came from permissions on the isolated
Rails storage volume. After correcting that fixture, the real upload completed
2/2 records with one new barcode and cleared its archive reference. Importing
5016298210989, adding dm+d 777 through the wizard, then confirming a refill of 7
persisted supply 35. Actual phone-camera evidence remains outstanding; camera
test doubles do not establish real-device acceptance. No merge or deployment is
authorised by this implementation record.

Implementation owner: `/root/astra_scanner_plan`, checkout
`/Users/damacus/.codex/worktrees/6702/med-tracker`, branch
`codex/scanner-stock-migration`, starting reference `949535c3e`. The coordinator
owns commits, publication and fixture cleanup. This source owner retains fixes
until the complete journey passes review and focused verification.

Focused evidence so far: API session 29775 failed on the absent lookup routes;
53178 passed the first five contract cases. Session 19782 passed six cases and
failed the pack-name/strength match case; that correction awaits verification.
Browser session 14757 passed manual entry through staged creation and camera
denial with manual fallback. Its admin-import route and selected-option cases
were RED. Session 85080 confirmed the refill dialog's accessible name; the
selected-option label needed correction. Archive extraction safety RED runs in
session 85188. These are intermediate results, not full-slice acceptance.

### Ownership handoff: Astra to Sol, 10 October

The owner requested lower token use and a routing change. Astra stops source/test
writes after this handoff. The coordinator will assign Sol as the sole writer in
the same checkout and branch above, still based on `949535c3e`. All implementation
is uncommitted. No PR, commit, push, merge, deployment or source freeze exists.
Do not restart discovery or remove approved acceptance criteria.

Sol accepted sole source/test ownership in the same checkout and branch. The
remaining work has defined Rails evidence and objective tests, while worker
failure and stock writes still need everyday engineering judgement. GPT-6 Sol
at its advertised medium default fits that work; the applicable weekly usage
window had 60% remaining at transfer. The coordinator retains review,
publication and fixture ownership. The next evidence is the queued-worker
failure tests and the Task runner's ambient-filter regression.

Uncommitted source inventory: shared lookup and matching/enrichment modules under
`src/models/care/medication_lookup*`; public API and browser finder controllers;
scanner/wizard JavaScript and Tera views; selected-option refill and initial-dose
helpers; NHS storage/parser/worker/admin modules; app registration and scheduler
configuration; Cargo/npm dependencies and scanner bundling; Rust/browser tests;
owned RustFS additions in Compose, Taskfile and migration test runners. Rails
sources are untouched. `.playwright-mcp/` and `docs/screenshots/scanner-migration/`
contain coordinator-owned reference evidence; preserve them.

Verified intermediate results:

- API session20290: all eight lookup tests pass, including pack/strength matching,
  case/form aliases, ambiguity, curated/local priority, trade-family and review
  evidence enrichment. Earlier enrichment RED was31956.
- Unit session75815: both archive extraction tests pass after entry-count RED
  85188. Limits include actual emitted bytes; forged size metadata was already
  rejected by zip9 and remains covered.
- Worker session58698: missing archive becomes failed, stale imports become
  failed after30minutes, and five-minute reconciliation is configured in all
  environments. Worker Task includes `--scheduler`.
- Session80873: four worker tests pass using automatically provisioned owned
  RustFS, including actual PostgreSQL queued upload, parsing, persisted barcode
  5016298210989/code777 and terminal object/reference cleanup. This proves the
  happy path, not read-back-before-enqueue or restart/failure completeness.
- Browser session14757 passed manual miss → current staged creation → refreshed
  medicine and camera denial/manual fallback. Selected-option refill label fix
  still needs GREEN. No persistent Loco review server/screenshots exist yet.

Current handles and immediate action:

- **Session 92170 finished: 4 pass, 2 RED** for
  `rtk task slice:test TARGET=care_api FILTER=nhs_dmd`. Its owned DB/storage are
  cleaned up. Enqueue failure and a missing stored object both leave status 0,
  expected 5. Fix these next. No implementation-owned job remains running.
- Runner session64171 finished10pass/1fail. Its diagnostic establishes inherited
  `SLICE_FILTER=unrelated_filter` overrides explicit `FILTER=nhs_dmd` in the Task
  environment; Cargo receives `unrelated_filter`, so storage is not provisioned.
  Fix ambient-filter handling, retain regression, rerun `task slice:test-runner`.
  Import tests must never silently skip without storage.
- Outputs remain in tool sessions. Resolver RED31956 full RTK log:
  `$HOME/Library/Application Support/rtk/tee/1791670055_task_sli_04d3a0.log`.
  No other owned application job runs; coordinator owns Rails51702 and its separate
  RustFS 49705 fixture/cleanup.

Outstanding implementation and proof:

1. Imports: read back uploaded bytes before enqueue; mark queue/get/config failures
   failed with useful sanitised messages; retry terminal cleanup without restarting
   failed imports. Add restart, concurrency, integrity, malformed archive, progress
   and cleanup-failure tests. Keep singleton lock/counters. `fail_run` propagates DB
   errors. Validate inherited parser GTIN cases. Admin browser success, CSRF,
   session/admin revocation and audit parity remain.
2. Wizard: current four-stage baseline is not complete approved parity. Implement
   saved fullpage/modal/slideover and launcher variants; structured time/date/day
   controls, PRN rules, Rails free-text taper plan, generated frequency/defaults;
   retained drafts. Replace generic duplicate error with explicit medicine/location
   choice and no writes. Lock/recheck final submit; preserve atomic person plan and
   initial dosage, idempotence and postcommit dosage summary.
3. Finder: fix metadata prefill (`dmd_type` differs from wizard's
   `dmd_concept_class`; custom products must retain system), text search/filter,
   related medicines/review display; `?refill=true` does not yet auto-open. Preserve
   public path/refill fields and check complete OpenAPI and enrichment-failure
   behaviour. CSV lookup works; preserve its import dependency.
4. Stock/browser: verify option additive result/replay/audit rollback, retain option
   on validation; adapt old rejection test without losing server protection.
   Complete camera cancellation/stale callbacks, ambiguity, unavailable/error,
   revocation/concurrency and desktop/mobile wrappers. Keep dialog accessible-name
   assertion; remove diagnostic attachment only after label fix passes.
5. Finish formatting, MSRV compatibility (AWS SDK needs1.94.1), frontend bundler
   task inputs and CI runner checks. Do not edit historical ledgers. Provide stable
   synthetic Loco listener/screenshots for coordinator's code/security and three UI
   judges, then frozen `task ci`. Do not claim real phone camera acceptance.

### Sol implementation checkpoint, 11 October

The earlier handles and outstanding list record the handoff state, not current
results. Sol remains the sole source/test writer; the coordinator still owns
commits, push, publication, independent reviews and owned-fixture cleanup.
The Loco implementation now includes both Medication Finder and Inventory Scan
stock entries, camera close/cancel fencing, text and barcode lookup, source and
review metadata, related authorised stock, a four-stage structured wizard,
saved modal/slide-over presentation, person launcher variants, atomic dose
options and stock, explicit duplicate choices, and additive selected-option
refill. Browser responses preserve the strict public lookup projections.

Focused evidence: `rtk task slice:test TARGET=care_api FILTER=medication_lookup`
passed 9/9, including the PostgreSQL enrichment fallback. `rtk task slice:test
TARGET=care_api FILTER=nhs_dmd` passed 13/13, including storage read-back,
queue and archive failures, real worker shutdown/restart with a persisted queued
release, 31-minute interruption without automatic rerun, concurrent uploads,
integrity failure and terminal cleanup. `rtk task frontend:test-browser
GROUP=care-scanner GREP='NHS import administration'` passed 4/4 on desktop and
mobile for platform authority, CSRF, queued upload and session/role revocation.
The grouped scanner refill browser run passed 4/4 for selected-option additive
stock, replay conflict and audit-failure rollback. The complete scanner browser
run passed 38/38 across desktop and mobile (session 54788). `rtk task
slice:test-runner` passed 11/11 (session 41274), `rtk task frontend:build`,
`rtk task fmt`, `rtk task lint` (session 63840), and `git diff --check` passed.
Sol froze application/test source for independent review after these results.
The named synthetic review stack is `mtloco-scanner-review-20261011`, with Loco
and its worker at `http://localhost:51703` (session 29038), PostgreSQL on
127.0.0.1:53372 and RustFS on 127.0.0.1:53631. A successful `/up` response
confirmed the listener. Its mode-0600 Fish environment file is
`/private/tmp/medtracker-scanner-review-20261011.fish`; do not publish its
contents. The fixture user is `persistence@example.test` with the standard
fixture password. The coordinator owns fixture cleanup and independent code,
security, UI and final CI review. Synthetic browser automation cannot prove
camera behaviour on a physical phone.

### Independent review checkpoint, 11 October

Delivery uses `team-slice-development` and `adaptive-model-routing`. Sol owns
implementation and fixes after the completed Astra planning handoff. The separate
subagent-driven-development workflow is not layered onto this slice. Sol also
handles the UI Professional and Accessibility reviews; the existing charter
requires the separate vanilla Astra judge. The latest account-wide usage check
reported 56% of the weekly allowance remaining; the other window was unavailable.

The source fingerprint in
`/private/tmp/medtracker-scanner-review-20261011/candidate.sha256` matched all 65
candidate files before review fixes. Devin CLI `swe-2-max`, session
`flourish-fortnight`, completed its source review in job 4934. Its report write
needed interactive permission, so the coordinator saved the exact report content
from the session export to `devin-verdict.md` in that directory. The review found
incorrect household links, supplement plan classification and nested ZIP cleanup,
plus smaller lookup and saved-default compatibility gaps. These need correction.

The vanilla Astra report is `/private/tmp/medtracker-scanner-ui-astra.md`; it
requires saved dosage and weekly weekday summaries. The UI Professional report
is `/private/tmp/medtracker-scanner-ui-professional.md`; it requires clearer
scanner next actions and an explanation of the required plan review. Accessibility
agent `/root/scanner_accessibility` is checking the actual UI next. Sol will fix
the consolidated findings, followed by affected re-review and final `task ci`.
Review screenshots are under `docs/screenshots/scanner-migration/`. No PR has
been published and the slice is not yet accepted.

Native Playwright clicks and keys work in an isolated browser process with the
required local execution permission. Reviewers reuse a private authenticated
fixture state. An existing test task reset the sign-in throttle only in the
owned synthetic database after repeated reviewer logins reached its limit.
No production account or authentication policy changed.

### Corrected candidate, 11 October

Sol fixed the code and UI findings with failing regressions first. Lookup now
uses working household links, normalised filters and distinct GTIN results.
The wizard retains schedule defaults and correct supplement plan classification,
and the confirmation reads saved dosage options. Nested archive files are removed
with their owning temporary directory. Manual medicines no longer inherit false
NHS provenance. Scanner actions, plan-review instructions and keyboard focus are
corrected, including stopping a pending camera without late callbacks stealing
focus.

Focused results: lookup 10/10, nested GTIN cleanup 1/1, complete scanner browser
44/44 on desktop/mobile, and final affected camera checks 8/8. Formatting, lint
and diff checks pass. The Vitamin PRN browser case verifies a saved routine
assignment, null manual dm+d system and retained schedule configuration.
The corrected source is frozen; review app session 16841 serves
`http://localhost:51703` with the same owned database and storage.

All three original UI judges passed their affected rechecks using native browser
input. Their separate reports and corrected screenshots retain the evidence and
limits. Explicit Stop returns focus to Start Scanner; camera failure focuses the
manual Barcode input. Physical phone-camera behaviour remains unverified.

The exact correction diff and candidate fingerprints are in
`/private/tmp/medtracker-scanner-review-20261011/`. After an automatic approval
block, the owner explicitly approved sharing the correction diff and relevant
private source. The same Devin session `flourish-fortnight` accepted every
correction and the wizard validation repair, with no new actionable findings.
The verdict is saved as `devin-recheck-verdict.md` in that directory.
Final local `task ci` session 30043 completed with a failed browser gate: all Rust
targets passed; 500 of 508 browser cases passed. Four existing journeys failed on
desktop and mobile: medication CRUD, locations and assignments still assume the
old one-page medication form, and medication detail expects dosage-option refill
to be rejected. Sol updated those journeys to the approved behaviour while
retaining validation, stale-write, saved-state and deletion coverage. The full log
is `/Users/damacus/Library/Application Support/rtk/tee/1791677447_task_ci.log`.
The focused checks exposed a real wizard regression: server validation returned
only a generic message. The fix reuses the existing linked error summary for
fields the wizard can show and focus, marks and describes invalid inputs, and
returns to their step with the draft intact. Errors for hidden fields or extra
dose options retain an alert without a broken link.
All eight affected desktop/mobile cases now pass in focused runs, including
assignment edits followed by recording a dose and checking reduced stock.
Focused validation checks passed 6/6, with the final hidden-frequency check 2/2.
Formatting, lint and diff checks pass. The original Accessibility judge passed
the changed threshold error state on desktop/mobile and the Name error on desktop,
including retained input, linked messages and keyboard focus. Its existing report
and `loco-accessibility-*-server-validation.png` screenshots retain the evidence.
Final `task ci` session 65738 passed on the frozen, reviewed source: 420 Rust tests
and all 510 desktop/mobile browser cases passed. The deliberate catalogue-capture
test remains ignored. The browser phase took 22.7 minutes. Full evidence is saved
at `/Users/damacus/Library/Application Support/rtk/tee/1791683595_task_ci.log`.
Documentation build and diff checks passed. Physical phone-camera verification
is tracked in [#2524](https://github.com/damacus/med-tracker/issues/2524).
The owned review listener and synthetic database/storage stack have been removed;
the earlier Rails reference and storage-probe stacks were already removed.
The reviewed candidate is ready for publication. No merge or deployment is authorised.

## C4: Add Medication wizard

Reference: `rails/app/components/medications/wizard/`, including step indicator,
basic information, dose/schedule, dosage options, supply and warnings, plus full
page/modal/slide-over wrappers. Inspect the actual orchestration before fixing
the step order; file names alone are not the user journey.

- [ ] Inventory entry points, steps, back/next/close controls, validation, retained
  values, selected-person context and final confirmation from rendered Rails.
- [ ] Port the staged flow and progress indicator using existing Loco medication,
  dosage, schedule and stock operations; complete missing necessary operations.
- [ ] Reuse structured timing inputs and generated readable summaries rather than
  requiring duplicate free-text frequency. Keep dose limits distinct.
- [ ] Preserve approved profile experiment selections for full page/modal/slide-over
  and current/context-aware launchers; coordinate with the profile parity subplan.
- [ ] Integrate scanner/manual entry without making camera access mandatory.
- [ ] Verify back/next preserves input, invalid steps do not create partial records,
  close/return preserves intended context, duplicate submission is safe and the
  created medication is immediately usable. Test all supported wrappers.

## Priority and acceptance

Start with C1 and C2, then C3 and C4. This is an implementation order, not removal
of scanner or wizard scope. Each checklist must link its source inventory,
failing-to-passing evidence, screenshots and PR before being marked complete.
Apply frontend-design, ui-professional, translate, accessibility and web-quality-audit
to find and fix issues. Inspect semantic names/states, keyboard shortcuts and
navigation, error/update announcements, target sizes, contrast and zoom/reflow.
Translate visible and accessible text together and set document language.
The owner does not require a dedicated screen-reader session; describe actual
automated/inspection coverage without claiming untested assistive-tech compatibility.
