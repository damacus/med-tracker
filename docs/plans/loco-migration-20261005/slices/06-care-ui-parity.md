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
