# Next journey step: record a dose and see stock/history

Begin after secure entry passes independent review and acceptance.

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
