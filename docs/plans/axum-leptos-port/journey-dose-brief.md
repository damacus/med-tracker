# Next journey step: record a dose and see stock/history

Begin after secure entry passes independent review and acceptance.

Active baseline: `2a8b4c1b`, clean and pushed. Secure entry passed 30 HTTP and
five browser cases with independent requirements and quality review.

## Current ownership

Under the user-approved parallel-writer charter:

- Test writer: `rust/contract-tests/tests/dose_write_api.rs`, contract fixture
  types, `scripts/contract_provision.rb`, selected contract Task/test-runner
  wiring, and `journey-dose-tests-report.md`. Existing full `doses.rs` remains
  the behaviour source; avoid rewriting unrelated setup workflows.
- Product writer: `rust/api/src/**`, API manifest/lock and
  `journey-dose-product-report.md`. Propose transaction, idempotency and
  permission boundaries before implementation. Ordinary persistence uses
  SeaORM. No production writes before recorded red.
- Independent reviewer: source/requirements review and
  `journey-dose-review.md` only, with early review of permissions and atomicity.
- Runner: Task execution and `journey-dose-runner-report.md` only. Run the
  final combined checks only after explicit joint freeze; capture complete
  input digests, fixture/image identity and owner-checked cleanup.
- Orchestrator: shared interface decisions, plan/ledger, first-party browser
  boundary design, integration and Git. Browser implementation ownership will
  be assigned after the API contract and session boundary are settled.

Same checkout with disjoint paths. No comments, unrelated changes, live data,
deployment, merge or Git operations by subagents. Read the Ruby skill for
Rails source/fixture work and use Serena discovery and Context7 as required.
Use Fish, `rtk`, and project Task commands. After two failed fixes, stop and
escalate the concrete discrepancy. Do not weaken tests or broaden into CRUD.

## Observable outcome

A signed-in user selects an authorised household and medication, records a
dose, and sees one history entry and the corresponding stock reduction.
Retrying the same request must not record or consume the dose twice.

## Bounded API work

Use the direct medication-take contracts in
`rust/contract-tests/tests/doses.rs`, beginning with
`medication_takes_create_filters_paginates_and_preserves_precision` and the
following privacy and invalid-input cases. Implement household-scoped
medication-take collection GET and direct POST before scheduled-occurrence
actions. Use exact decimal arithmetic for dose and stock.

Direct POST must support both schedule and person-medication sources, tracked
dosage inventory and the existing timing/overlap rules. Blanket unsupported
responses for valid direct takes are not acceptance of this slice. The
separate scheduled-occurrence action routes remain subsequent work.

Those existing tests create medications, people and assignments through
other API routes. For the first Rust dose slice, provision equivalent
disposable starting records through the existing fixture builder. Keep dose
creation, history, stock and retry assertions against public HTTP. Do not
expand this slice into unrelated CRUD merely to satisfy test setup.

Preserve current membership and person-access checks, role denials, source
and stock-medication binding, time validation, audit attribution and response
metadata. Record the take, stock change and audit atomically. Cover a rejected
write and a contending duplicate request without partial writes or double
stock consumption. Confirmed Rails defects require intended-behaviour tests.

## Browser work and ownership

The test writer first specifies the household/medication/dose/history flow
and demonstrates its failure. The product writer adds the corresponding
Leptos pages using the shared API boundary. Keep separate file ownership;
review and runner remain independent. Use the existing internal Compose
browser sidecar and record desktop/mobile evidence.

Secure entry currently begins from a registered mobile client's OAuth
authorisation request. Standalone web dashboard sign-in remains a gap: the
web journey must establish its supported first-party session/API boundary
before claiming an end-to-end browser medication workflow.

Complete the usable flow before moving to the next feature family. This
slice does not close offline/PWA delivery, all scheduled dose actions,
migration, or performance acceptance.
