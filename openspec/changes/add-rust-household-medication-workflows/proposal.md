# Proposal

## Why

The Rust browser app exposes medication viewing and dose recording but lacks everyday household management journeys. Extend the Rust delivery tracked by [issue #2299](https://github.com/damacus/med-tracker/issues/2299) while Rails remains operational.

## What Changes

- Deliver authorised People and Locations list/detail/create/edit journeys.
- Deliver medication create/edit, stock adjustment/order/receipt and immediate persisted read-back.
- Deliver assignments and all seven schedule types, including multi-step taper editing and pause/resume.
- Share accessible native forms, household navigation, API-derived action affordances and five-language rendering.
- Preserve existing API/session, audit, idempotency, private-cache and medication safety boundaries.
- Execute a two-hour assessed run with parallel module owners, separate tests and independent review.

Non-goals: production cutover, deployments, merges, Rails retirement, Finder, reports, reviews, general administration, location deletion/membership administration and new offline capabilities. Authentication parity is a separate linked change.

## Capabilities

### New Capabilities

- `rust-household-management`: Complete first-party household management journeys with authorised forms and persisted results.

### Modified Capabilities

None. Existing domain safety requirements remain authoritative.

## Impact

Rust browser/API adapters, Leptos components, locale adapter, contract/browser tests and delivery records. Existing versioned API remains the business-rule authority. Additive action capabilities may require authoritative OpenAPI updates. Rails product code and live data are unchanged.
