## Context

The direct medication-take endpoint already provides the required domain path: it resolves a `Schedule` or `PersonMedication` through policy scope, checks `take_medication?`, and invokes `MedicationAdministration::RecordDose`. That service is the sole normal creator of immutable administration history and owns timing, dose, inventory, stock-source, lifecycle-lock, audit, and change-record behavior.

The sync batch endpoint currently handles only medication and health-event updates and deletes inside one `requires_new` transaction. It assumes every operation has an existing record and an ETag. Medication-take creation has different input, authorization, idempotency, and failure semantics, so adding it directly to the existing update/delete branches would blur those contracts.

`MedicationTake.client_uuid` is protected by a partial unique PostgreSQL index. A uniqueness violation aborts the current PostgreSQL transaction until rollback, which means a concurrent replay cannot be recovered by querying from inside the still-open batch transaction.

## Goals / Non-Goals

**Goals:**

- Add exactly one create-only sync operation for queued medication takes.
- Reuse the direct endpoint's authorization and canonical dose-recording boundary.
- Make normal and concurrent replays idempotent without double side effects.
- Preserve all-or-nothing behavior across mixed batches.
- Return stable machine-readable errors without disclosing household health data.
- Keep the change small enough to verify through focused request and service tests.

**Non-Goals:**

- Generalizing sync into a new framework or rewriting the existing update/delete paths.
- Supporting medication-take update or delete.
- Changing medication safety, inventory, authorization, audit, or tenant rules.
- Updating OpenAPI schemas or generated client types, which remain in #1656.
- Adding offline schedule, assignment, medication, or health-event creation.

## Decisions

### 1. Give medication-take creation a dedicated operation path

`BatchesController` will continue to own the HTTP envelope, operation ordering, policy calls, and outer transaction. Its dispatcher will route only `action: create` plus `resource_type: medication_take` to a small `Api::Sync` application service. The service will validate and normalize the medication-take attributes, invoke `MedicationAdministration::RecordDose`, and translate its domain outcome into a batch-safe result or typed failure.

The controller will resolve sources through `policy_scope(Schedule)` or `policy_scope(PersonMedication)` and authorize `take_medication?` before invoking the service. Existing-record operations will retain their current `id` and `if_match` contract; medication-take creation will use `attributes` and will not require an ETag.

Alternative considered: add all behavior as controller private methods. Rejected because error mapping, input validation, and idempotency retry behavior need focused service coverage and would make the batch controller a second medication-administration boundary.

Alternative considered: replace the whole batch dispatcher with a generic command framework. Rejected as unrelated scope and unnecessary for one explicit create operation.

### 2. Treat `client_uuid` as the idempotency identity

`client_uuid` will be required and blank values will fail before source lookup or dose recording. Before recording, the controller will search the authorized medication-take scope for the UUID and authorize the matching take with `create?`. A visible and authorized match will be returned as a replay without revalidating submitted dose attributes, matching the direct endpoint's current behavior.

Each successful medication-take operation result will include the existing batch result fields plus `replayed: false` for a new take or `replayed: true` for an existing take. This lets an offline client distinguish acknowledgement of an earlier administration from a new write while retaining one HTTP 201 response for the batch.

Alternative considered: compare every replayed attribute with the stored take. Rejected because the direct endpoint currently treats the UUID itself as the identity and changing that rule only for sync would create conflicting idempotency contracts.

### 3. Retry the complete transaction after a concurrent UUID race

The medication-take operation service will recognize the named `index_medication_takes_on_client_uuid` uniqueness constraint and raise a dedicated retry signal. That signal will escape the transaction immediately; no query will run in the aborted transaction. Because `MedicationAdministration::RecordDose` also serializes household lifecycle writes, a losing request can instead observe the winner during model uniqueness validation and return `create_failed` without aborting PostgreSQL. That persistence outcome will use the same one-time retry signal. After rollback, the controller will restart the entire batch once, rebuilding the results array and re-evaluating all authorization, ETags, and domain rules.

On the retry, the winning take is found and returned as a replay. If it remains invisible and the named constraint is hit again, the batch will return HTTP 409 with `idempotency_key_unavailable`. A repeated non-constraint persistence failure will return `medication_take_invalid`. Other uniqueness failures will not be converted and will retain normal exception handling.

This whole-batch retry is safe because the losing attempt has rolled back every earlier operation and medication side effect. If a sibling resource changed concurrently before the retry, its existing ETag check will produce the normal `sync_conflict` rather than silently overwriting it.

Alternative considered: rescue `ActiveRecord::RecordNotUnique` and query inside the transaction. Rejected because PostgreSQL leaves that transaction unusable until rollback; this is the database-failure pattern the design must avoid.

Alternative considered: add a new advisory lock per UUID. Rejected because the unique index already provides the final correctness boundary and a bounded whole-transaction retry preserves the existing idempotency model with less locking machinery.

### 4. Keep authorization indistinguishable across inaccessible sources

Source lookup will use the active household's existing policy scopes. A stale, cross-household, or otherwise invisible source will therefore produce the existing HTTP 404 `not_found` envelope. A visible source without the person's `record` grant will produce the existing HTTP 403 `forbidden` envelope. Replay lookup will use the authorized medication-take scope and `MedicationTakePolicy#create?`.

The service will receive only a resolved, authorized source. It will not perform unscoped source or take lookup, and it will not broaden owner, administrator, carer, parent, self, or member permissions.

Alternative considered: distinguish stale, cross-household, and hidden records in the response. Rejected because that would disclose record existence across an authorization boundary.

### 5. Map domain failures to a small public error taxonomy

The batch error type will carry a public code and sanitized message so the controller can use the normal API error envelope. The new operation will use:

| Failure | HTTP | Code |
| --- | ---: | --- |
| Missing or malformed take input | 422 | `medication_take_invalid` |
| Medication-take update/delete or unsupported create shape | 422 | `sync_operation_unsupported` |
| Stale, hidden, or cross-household source | 404 | `not_found` |
| Visible source without `record` access | 403 | `forbidden` |
| Selected tracked stock cannot fulfil the dose | 422 | `medication_stock_unavailable` |
| Cooldown, daily limit, overlap, pause, or timing rejection | 422 | `medication_timing_conflict` |
| UUID collision that cannot be safely replayed | 409 | `idempotency_key_unavailable` |

Other invalid dose/source-selection outcomes will use `medication_take_invalid`. Messages may include the zero-based operation index but will not echo UUIDs, portable IDs, medication or person names, dose values, notes, or inaccessible record data.

Alternative considered: expose only the existing generic `unprocessable_content` code. Rejected because offline clients need stable failure categories and the issue explicitly requires stable errors.

### 6. Rely on the existing transaction for all side effects

The canonical service will run inside the current batch transaction. Medication-take persistence, inventory decrement, PaperTrail versions, API change records, and tombstones therefore commit or roll back with sibling operations. Any after-commit delivery runs only after the complete batch commits.

No model callback, raw insert, bulk update, or direct stock mutation will be added. This preserves the sole-creator architecture check around `MedicationAdministration::RecordDose`.

## Risks / Trade-offs

- **[A replay may contain attributes different from the original request]** → Treat the UUID as authoritative, return only the authorized stored take, and document this as parity with the direct endpoint.
- **[A full-batch retry repeats application work]** → Retry at most once for the named UUID constraint or the canonical service's serialized persistence failure; the failed attempt is fully rolled back before re-execution.
- **[A sibling ETag can become stale during retry]** → Preserve normal `sync_conflict` behavior instead of weakening optimistic concurrency.
- **[Global UUID uniqueness can collide with an inaccessible take]** → Return a generic 409 without confirming what owns the UUID.
- **[New error codes extend the runtime API before generated types change]** → Keep the change runtime-only as required by #1675 and coordinate documentation/types through #1656.
- **[Controller and service behavior could drift from the direct endpoint]** → Assert the canonical service call and shared policy behavior in focused tests; do not duplicate medication rules.

## Migration Plan

No database migration or backfill is required. Deploy the additive runtime path behind the existing authenticated sync endpoint. Existing clients continue to use update/delete operations unchanged, while newer clients can submit the new create shape.

Rollback is code-only: remove the new dispatcher branch and service. Medication takes already committed through this path remain valid immutable history and require no data rollback.

## Open Questions

None. The explicit `replayed` result flag and public error-code table are proposed contract decisions for review before `/opsx:apply`.
