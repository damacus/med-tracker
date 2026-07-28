## Why

Offline clients can queue medication takes, but the sync batch endpoint cannot currently submit them after reconnecting. Sending the same queued dose through the direct API is idempotent, while retrying a broader sync batch has no equivalent safe path and risks either losing the dose or recording it twice.

Origin: [GitHub issue #1675](https://github.com/damacus/med-tracker/issues/1675)

## What Changes

- Accept `create` operations for the immutable `medication_take` resource in sync batches while continuing to reject update and delete operations for that resource.
- Require a `client_uuid` and replay the existing visible medication take when that idempotency key has already been used, without a second dose, stock decrement, audit entry, or change record.
- Apply the same source visibility, person-level `record` authorization, timing, dose, stock-source, inventory, audit, change-record, tenant, and household rules as the direct medication-take endpoint.
- Preserve all-or-nothing batch behavior when a medication take or any sibling operation is invalid, unauthorized, stale, unavailable, or conflicts with medication safety rules.
- Return stable, PHI-safe failures for unsupported actions, stale or cross-household sources, unavailable stock, and timing conflicts.

### Non-goals

- Documenting or generating client types for the expanded sync contract; that remains tracked by #1656.
- Adding offline mutations for schedules, assignments, medications, health events, or medication takes other than creation.
- Changing medication-take mutability, medication safety rules, or the direct medication-take endpoint's public behavior.

## Capabilities

### New Capabilities

- `medication-take-sync`: Queued medication takes can be created and safely replayed as part of an atomic sync batch.

### Modified Capabilities

None. This repository does not yet contain an OpenSpec capability for sync batches.

## Impact

- Extends the runtime contract of `POST /api/v1/households/:household_id/sync/batches`.
- Reuses `MedicationAdministration::RecordDose`, existing Pundit policies, medication-take immutability, and the `client_uuid` uniqueness constraint.
- Adds request/service coverage around batch creation, replay, authorization, safety failures, concurrency, and transactional rollback.
- Does not change the OpenAPI document or generated client types in this change.
