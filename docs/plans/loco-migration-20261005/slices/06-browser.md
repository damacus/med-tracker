# Complete browser workflows and themes Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Replace the legacy browser application with complete, accessible Tera/daisyUI journeys.
**Architecture:** Browser controllers call accepted shared operations and render Tera projections.
Small application scripts own interaction and offline outbox behaviour; CSS owns appearance.
**Tech Stack:** Loco/Tera, daisyUI theme exports, maintained browser tooling and service-worker APIs.
**Spec:** [implementation-spec.md](../implementation-spec.md), captured profile/notification/theme/font PR inputs.

## Plan-to-progress mapping

These are subdivisions of existing slice 06 items, not new scored milestones.
The report keeps its stable IDs. A partial PR cannot complete an entire row.

| Existing report item | Owning task and detailed plan | Current evidence and remaining work |
| --- | --- | --- |
| s06-i01 Shell/themes | B1; profile P10–P11 consumes theme engine | Published baseline; preserve it and verify the new profile entry point |
| s06-i02 Care pages | B2a; care C1 person cards, C2 medication details, C3 inventory presentation, C4 wizard; timing follow-up below | Care baseline exists; layout PR #2474 published with local checks. Full Rails interaction parity and readable timing remain planned |
| s06-i03 Profile/settings | B2b; profile P01–P28 | Local operations partially work; complete four-tab composition and behaviour unimplemented. Auth-owned security integration remains dependent |
| s06-i04 Admin/support | B2c | Existing household administration is partial evidence; broader admin/support retains auth-first sequencing |
| s06-i05 Data/scanner/reviews | B2d; care C3 scanner and C4 entry handoff; profile P22 export presentation | Reports PR #2470 merged. Scanner-to-add, scanner-to-restock, catalogue import and review freshness remain incomplete |
| s06-i06 Accessibility | B1/B2 across both new subplans; timing/localization follow-up below | Single-form audit and selected UI checks exist; find/fix/verify whole-journey keyboard, semantics and translation gaps |
| s06-i07 Avatars | B2b; profile P06–P07 | Local storage/browser evidence partial; cleanup/retry/mobile verification remains, Gravatar permission pending |
| s06-i08 Live care | B3 live-update portion; preserve across care C1–C4 | 54 selected cases pass; passive session renewal blocks acceptance; full checks/publication remain |
| s06-i09 Offline | B3 offline portion | Deferred from first release, retained in full goal; not resumed by parity planning |

Dependency mapping: care mutations use slice 04 shared operations. Catalogue
resolution/private data/export contracts map to slice 05 A3; dm+d import,
reconciliation, review refresh and push delivery map to slice 07 W1–W3. Scanner
is accepted only when both user journeys and their necessary operations work.
Profile security/tokens/closure consume slice 03 services; lifecycle/export work
follows the agreed auth-first sequence. Do not dispatch paused unrelated work or
double-count a single passing check as two completed capabilities.

## Profile parity subplan

The [care UI parity subplans](06-care-ui-parity.md) cover person pages, medication
details, inventory/scanner and the Add Medication wizard. These require working
ports of the established Rails interactions, not audit-only deliverables.

The owner requires the complete Rails profile layout and every reachable item.
Follow [Profile page parity](06-profile-parity.md), including the four tabs,
settings sheets/dialogs, expandable groups and profile-hosted theme picker.

## Owner follow-up: understandable timing and accessibility — 7 October 2026

- [ ] Replace overlapping Frequency, Dose cycle and Schedule type presentation
  across dose-option forms/lists, treatment forms and treatment cards. Follow the
  Rails interaction: choose structured fields and generate a plain-language
  summary, without requiring the same information to be entered again as text.
  Keep planned timing distinct from dose limits. Present limits as maximum doses
  per day/week/month and minimum hours between doses; do not label weekly or
  monthly limits as daily. Preserve existing saved instructions and semantics.
- [ ] Reuse the useful prior art in
  `rails/app/components/schedules/frequency_preview.rb` and
  `rails/app/components/person_medications/card/timing_status_component.rb`.
  Adapt the interaction to the current themes rather than copying legacy labels.
- [ ] Run a dedicated accessibility audit of representative new browser journeys
  and their error, dialog, stale-draft and live-update states, using the local
  `accessibility` skill alongside `web-quality-audit`. Cover keyboard-only use,
  visible and restored focus, screen-reader names and announcements, understandable
  labels, contrast across themes, zoom/reflow and touch targets. Record automated
  evidence separately from manual and assistive-technology checks, with explicit
  gaps where those checks cannot be completed. The existing single-form Lighthouse
  accessibility score of 100 does not establish whole-application conformance.

Find and fix the discovered defects, verify them, obtain independent review and
publish PRs; an audit report alone is not completion. Per the owner, no dedicated
screen-reader session is required: use standards-based source inspection,
rendered accessibility-tree checks and browser automation without claiming
screen-reader compatibility tested. Include keyboard shortcuts/navigation and
language switching for visible and accessible text, plus document language.

Current source checkpoint: reports already select translated labels through
`report_pdf::locale` / `translations` and pass `lang` into the shared layout.
Their locale files cover English, Welsh, Irish, Portuguese and Spanish. Other
care templates currently use literal English strings and the layout defaults to
`en-GB`; this is not evidence of an application-wide language picker or translated
accessible names. Reuse the existing locale approach where suitable, inventory
the supported selection/persistence contract, and translate visible and accessible
strings together before marking the cross-journey requirement complete.

These are recorded follow-up requirements, not implemented or audited outcomes.

## Resumed delivery decisions — 7 October 2026

The owner resumed slice 06 in the isolated `loco-browser` worktree on branch
`codex/feat-loco-browser-experience`, based on main
`1c3c3da954fb50370371fd019b924a93a547c2a7`. Preserve the existing Tera/daisyUI
shell, care operations and merged report journeys from PR #2470.

Owner Q&A established these additional requirements:

- A dose or stock change appears on another authorised connected foreground
  page within one second. Preserve unsaved forms and show a stale-data notice.
  Show lost connections, catch up after reconnect, and refresh background tabs
  when reopened. The implementation transport is an engineering decision.
- This team owns missing domain and worker operations necessary to complete
  these browser journeys. Missing operations are dependencies to implement,
  not a reason to accept placeholder pages or defer required first-release work.
- Authentication and security implementation retain their existing owner.
  Do not edit the active identity worktree or resume its paused decisions.
  Consume verified interfaces and preserve the separately agreed sequence for
  platform administration/support and household exports/lifecycle after auth.
- Existing first-release exclusions remain: offline capture/replay, portable
  imports, native acceptance, FHIR/SMART, MCP and optional AI/provider lookup.
  Retain them in the full goal. No merge, deployment or live-data action is authorised.

The owner confirmed the shared brief and authorised implementation on 7 October.
Discovery reports are
`/private/tmp/medtracker-s06-care-discovery.md` and
`/private/tmp/medtracker-s06-data-discovery.md`. They describe the inspected main
revision; their initial five-second recommendation and outside-slice dependency
boundary are superseded by the owner answers above.

Proceed through coherent deliveries using one implementation writer, independent
review, and one costly verification lane:

1. Live care updates and remaining care actions, using existing tenant-scoped
   operations and projections; include reconnect, access withdrawal and draft tests.
2. Profile details/preferences and avatars; integrate security/session/device
   controls through the existing auth owner's verified interfaces.
3. Camera/manual scanner and catalogue resolution, review list/status journeys
   and required background refresh; reuse merged PDF pages and retained worker work.
4. Complete platform/support and data/lifecycle browser journeys with their
   required operations, respecting the previously approved auth-first sequence.
5. Verify accessibility and complete desktop/mobile journeys across the finished
   browser application, including denial, validation and immediate-usability cases.

Before each delivery, record exact owned paths, reused interfaces, behavioural
RED test, focused Task commands and report/review paths in its file brief.
Shared registration, schema, identity and fixture changes need explicit ownership
in that brief. Freeze the candidate for verification. Run relevant focused Tasks
and final `task ci`, meter CPU/RAM for full suites, obtain the required independent
Devin review, and publish scoped PRs with passing hosted checks. Record outcomes
in the existing progress ledger and render the authoritative
`/private/tmp/medtracker-progress-report.md` to its existing HTML output.

## Global constraints

### Care-page layout correction — 7 October 2026

The owner prioritised layout corrections before further profile implementation.
Keep secondary actions, especially Cancel, at their natural width on desktop and
mobile. Group related short fields in rows where their labels remain readable.
Lead with identity and dose, keep dose timing and limits together, group stock
settings, and give secondary details and destructive actions less prominence.
Apply this hierarchy across medication, dose-option, treatment, person and location
journeys. Use retained Rails layouts selectively where they suit the current themes.

Hours use whole-number controls and integral values display without decimal zeros.
Preserve existing fractional metadata without rounding; unrelated edits must not
silently change a stored interval. Keep barcode as record metadata, outside ordinary
care forms and summaries, and prove that saving other fields retains it.

Use frontend-design and web-quality-audit, with rendered desktop/mobile evidence,
keyboard/focus checks, contrast and overflow measurements. The profile worktree is
preserved at its test-only avatar-cleanup checkpoint while this correction proceeds.

The 6 October decision supersedes historical appearance and URL fidelity below:
use daisyUI, the same colour schemes and clear Loco routes. Exact Rails pixels,
CSS values/geometry and theme-export fidelity are unnecessary. Preserve required
information/actions and accessibility. PDF appearance must still match. Existing
push subscriptions may require re-enrolment. Pre-cutover offline queues need not
migrate; future offline capture/replay and normal push behaviour remain required.
Offline capture/replay is deferred from the first production gate, as subsequently
approved, while remaining in the complete goal. Automatic live dose/stock updates,
scanner, reviews, avatar uploads and complete device/session management are required
before first production.

Implement each journey against its locally verified operation interface. Complete
care/identity and relevant API/workers remain acceptance requirements, rather than
blocking independent shell/theme or journey implementation. The user's Tera/daisyUI migration
instruction supersedes Rails-only Phlex rules for the Loco UI; retain Phlex within Rails rollback.
Keep the same colour schemes, licensed fonts and usable theme preferences; fresh
daisyUI themes are allowed. No read-only journey substitute.

## Review focus

Initial paint matches saved/system appearance (B1). Keyboard focus returns after dialogs/tabs (B1).
Failed mutations preserve input and show associated errors (B2). All notification switches persist (B2).
Offline replay after permission revocation cannot write, and reconnect retries cannot double-dose (B3).

### B1: Establish the accessible shell and existing themes

**Files:** Create `assets/views/layouts/base.html`, `assets/views/components/{tabs,dialog,form_errors}.html`,
`assets/static/css/{app,themes}.css`, `assets/static/js/{appearance,interaction}.js`,
`tests/browser/shell.spec.mjs`, `scripts/migration/loco-browser.mjs`; modify `Taskfile.yml` and view initializer.
**Interfaces:** Produce `slice:browser GROUP=<group>` with actual owned Loco, desktop/mobile viewports
and screenshots. `appearance.js` reads/writes `med-tracker-theme` and `med-tracker-appearance`;
rendered pages provide associated labels/errors and semantic controls.

- [ ] Test every existing palette and Light/Dark/System, including before first paint and live OS change;
  test keyboard tabs/dialog focus. Assert `saved_palette_restored == true`,
  `system_change_applied == true`, `focus_after_close == triggering_control`.
- [ ] Register the browser Task with its test and run `rtk task slice:browser GROUP=shell`; record RED.
- [ ] Export existing themes with the official daisyUI theme creator; commit CSS directly.
  Preserve fonts/licences from captured inputs; implement accessible interaction with maintained patterns.
- [ ] Run shell checks at desktop/mobile and capture `docs/screenshots/loco-shell-{desktop,mobile}.png`;
  verify contrast, no horizontal overflow, focus and initial appearance in the actual UI.
- [ ] Review and commit `feat(ui): preserve accessible shell and existing themes`.

### B2: Port complete browser operations by journey family

**Files:** Create `src/controllers/browser.rs`, `src/controllers/browser/{care,profile,admin,data}.rs`,
`assets/views/{care,profile,admin,data}/` templates and browser tests listed below; modify route registration.
**Interfaces:** Each controller exposes `routes() -> Routes`; mutations consume the same care/integration
named model methods and the existing validated tenant transaction. Views receive
preloaded projections, never query the database.

Each row is an independently reviewed task using the five checkbox steps below. Complete all rows;
a navigation link or list page is not a successful write journey.

| Task | Templates/test file | Complete operation and acceptance assertions |
| --- | --- | --- |
| B2a Care | `care/`; `tests/browser/care.spec.mjs` | Create/invite household, accept invitation, create/grant person, medicine/dosage/stock order/receipt, schedule/assignment/pause/resume, take/correct dose; exact stock/audit once; role/capacity denial; supported person-scoped schedule edit |
| B2b Profile | `profile/`; `tests/browser/profile.spec.mjs` | All four profile tabs; edit details/avatar, password/MFA/passkeys, API tokens/sessions/devices, all five notification preferences; validation retains input; revocation takes effect immediately |
| B2c Admin/support | `admin/`; `tests/browser/admin.spec.mjs` | Create/update/retire managed records, duplicate/validation handling, immediately usable records, timed support access, owner promotion and household retirement with denied-role cases |
| B2d Data | `data/`; `tests/browser/data.spec.mjs` | Lookup/reviews, import status/retry, authorised export/history/PDF download and signed attachment flows; external failure/expiry shown accessibly; report timezone boundaries |

- [ ] Write each row's successful write, invalid input and denied-role browser cases; inspect actual
  persisted state. Assertions include `mutation_visible_after_reload == true`,
  `duplicate_side_effects == 0`, `denied_mutation_committed == false`.
- [ ] Run `rtk task slice:browser GROUP=<care|profile|admin|data>` for the row and record the missing journey.
- [ ] Implement that row's standard routes/controllers/templates via accepted shared operations;
  preserve existing labels, contract routing and error feedback. Never bypass permissions for rendering.
- [ ] Run the row's desktop/mobile browser checks, relevant domain tests and root CI; capture actual
  `docs/screenshots/loco-<group>-{desktop,mobile}.png` and verify keyboard operation.
- [ ] Review and commit each row independently; update its evidence in the HTML report when accepted.

### B3: Preserve offline dose capture, replay and live updates

**Files:** Create `assets/static/{service-worker.js,manifest.webmanifest}`, `assets/static/js/offline.js`,
`tests/browser/offline.spec.mjs`; modify care templates for pending/conflict feedback.
Read existing Rails PWA/offline JavaScript and sync/replay contracts.
**Interfaces:** Offline outbox stores the existing client UUID/payload identity and sends through C3's
accepted replay API. Personal caches/outbox are isolated by account and cleared according to existing logout policy.

- [ ] Test airplane-mode capture, browser restart, reconnect twice, permission withdrawal before upload,
  account switch and partial conflict. Assert `dose_count_after_two_replays == 1`,
  `revoked_pending_write_committed == false`, `other_account_sees_cached_health_data == false`.
- [ ] Run `rtk task slice:browser GROUP=offline`; record missing/offline safety failures.
- [ ] Implement service-worker cache/outbox/replay using proven browser capabilities and C3 contracts;
  retain pending/conflict states and existing realtime behaviour from source inventory.
- [ ] Run restart/offline tests, API replay tests and full browser suite; verify real UI feedback/screenshots.
- [ ] Obtain privacy/replay review, integrate and publish; reconcile every browser capability before completion.

**Done:** All journey families, appearance, accessibility, security settings, notifications and offline
replay pass actual desktop/mobile UI checks through the Loco server.
