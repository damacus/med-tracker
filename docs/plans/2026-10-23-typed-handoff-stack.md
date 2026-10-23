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
  the paired mature browser runs. Layers 3–5 remain planned.
- The final scope audit also tracks two separate areas outside this stack:
  app-token issuance and browser review-prompt editing. Browser occurrence forms
  belong to layer 4; report reads and review-prompt PDF conversions belong to
  layer 5. Completion of these five areas does not mean every application use
  of JSON has been removed.
