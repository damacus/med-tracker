# Sync read contract tests

## Scope

Cover only `getSyncSnapshot` and `getSyncChanges` from the fixed OpenAPI document. Reuse the provisioned household and owner API session; make no data mutations.

## Existing coverage

`rust/contract-tests/tests/sync.rs` already exercises manager and view-scoped snapshots, hidden and foreign record filtering, event ordering, inclusive cursors, tombstones, dose-outcome projection, authentication, foreign-household denial, and missing or malformed cursors.

## Remaining checks

- Assert the snapshot and change-feed response envelopes and their closed top-level schemas, required collections, timestamp formats, and feed row shapes.
- Assert both reads return the documented not-found response for a valid but absent household ID.

## Out of scope

Do not test sync batch writes, mobile v1 export, or duplicate the existing domain-specific sync visibility and cursor scenarios. No runner or production changes are part of this batch.
