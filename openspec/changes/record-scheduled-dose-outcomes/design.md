## Context

The dashboard and reminder queries currently derive expected doses from `Schedule` and infer completion from counts of `MedicationTake`. A take has no stable occurrence identity, so the same absence represents both an unexplained miss and a deliberate non-administration. `MedicationTake` is immutable, decrements stock through the canonical administration service, and must continue to mean an administered dose.

This change follows `record-medication-pause-periods`; expected occurrences must not exist inside known pauses. A later approved change extends the same record to direct routine assignments, so the schema must preserve the concrete two-source boundary from ADR 0006 without exposing that upper behaviour yet.

## Goals / Non-Goals

**Goals:**

- Derive stable formal schedule occurrences without pre-generating future database rows.
- Persist explicit outcomes and link taken outcomes to immutable takes.
- Serialize competing outcomes and retain auditable corrections.
- Give dashboard, notifications, reports, API, sync, and portable data one outcome interpretation.

**Non-Goals:**

- Generate occurrences for direct assignments in this change.
- Treat PRN/as-needed medication as expected work.
- Backfill guessed occurrence links for historical takes.
- Change schedule frequency, dose-cycle, stock, or timing rules.

## Decisions

### Derive pending occurrences and persist only outcome state

Add a household-scoped `MedicationDoseOccurrence` with nullable `schedule_id` and `person_medication_id` foreign keys plus an exact-one-source database constraint. This change permits only `Schedule` writers; the upper change enables the already constrained direct-assignment source.

The stable natural identity is source plus `window_starts_on` plus one-based `position`. A formal schedule uses each applicable local date as its window. The row also snapshots nullable `scheduled_at`, outcome (`open`, `taken`, or `not_taken`), optional not-taken reason/note, resolution time and membership, portable ID, and an optional unique `medication_take_id`.

Pending occurrences are value objects produced by one query from schedule rules, pause periods, existing outcome rows, and compatible takes. Do not write rows while rendering a dashboard or reading an API. Persist a row only on first resolution; reopening keeps the row with `open` outcome so its stable identity and audit history survive.

Use separate composite unique indexes for each concrete source. Check constraints require a take link only for `taken`, forbid a take link for `not_taken`, and limit reasons/notes to the not-taken state.

Rejected alternatives:

- A fake zero-dose `MedicationTake` would corrupt administration and stock semantics.
- Pre-generating all future occurrences would create unbounded writes and complex schedule-edit cleanup.
- Separate taken and not-taken tables would require cross-table uniqueness that PostgreSQL could not enforce simply.
- A polymorphic source would discard concrete household-safe foreign keys.

### Use one occurrence resolver and the canonical dose recorder

An occurrence query returns an opaque deterministic key containing no clinical text. The server decodes it into source portable ID, window, and position, then re-derives the occurrence under the active household and person policy scope. Clients do not construct or interpret the key.

A resolver service locks the source and any persisted occurrence, revalidates applicability and due time, and writes one outcome transactionally. Database uniqueness is the final concurrency guard. Repeated idempotent requests return the committed result; a different competing outcome returns an already-resolved conflict without disclosing hidden data.

Marking taken calls `MedicationAdministration::RecordDose` inside the resolver transaction and then links the immutable take. It never writes stock directly. Marking not taken writes no take and triggers no stock mutation.

Only a not-taken outcome can reopen. Reopen changes the current state to `open` and clears current reason/note/resolution fields while PaperTrail retains the former values. Replacing not taken with taken uses the normal recording path. A linked take cannot reopen, edit, or delete.

### Preserve legacy take behaviour without fabricated links

Do not backfill occurrence rows for existing takes. For read calculations, order compatible unlinked takes by `taken_at` and ID and allocate them to the earliest unresolved applicable occurrence in the existing dose window. Persist no inferred link.

Before accepting not taken, the resolver runs the same allocation and rejects an occurrence already satisfied by an unlinked take. New web and compatible API take actions provide the occurrence key and create an explicit link. Older valid clients may omit it and keep current behaviour.

### Make consumers use outcome categories

Dashboard and reminder queries consume the shared occurrence projection. They advance to the next unresolved position and suppress both taken and not-taken occurrences. Missed-dose jobs schedule and deliver only for unresolved overdue occurrences.

Reports expose counts and entries for taken, not taken, and unexplained overdue outcomes. Not taken remains in the expected population but is not labelled an unexplained miss. Smart Insights uses only unexplained misses for missed-dose patterns. History shows the supplied reason/note and correction state without presenting clinical advice.

### Add bounded additive interoperability

Add a capability-advertised occurrence collection filtered by person and a bounded date range, with a conservative maximum range enforced server-side. Add idempotent not-taken and reopen operations. Add an optional occurrence key to direct and queued take creation; omission preserves the existing `/api/v1` contract.

Sync snapshots and changes carry persisted occurrence rows. Sync batches support idempotent not-taken creation and ETag-protected reopen, using the same resolver. Portable v2 exports persisted rows after their sources and restores them after medication takes so take links resolve without replaying stock. Portable v1 remains unchanged.

OpenAPI defines stable operation IDs, schemas, reason enums, error codes, and capability metadata. Generated Kotlin and Swift clients are regenerated from that contract. Public errors contain no raw source IDs, medication names, notes, or inaccessible occurrence details.

### Keep authorization with the person

Reading occurrences uses view access. Recording taken or not taken uses record access. Reopening a resolved outcome uses manage access. Household owner/administrator behaviour remains whatever the existing source policies grant; the new policy delegates to the source person instead of creating a global permission.

PaperTrail records outcome state changes with request actor context. Audit projections treat the occurrence as the source fact; notification and reporting code never decide authorization.

## Risks / Trade-offs

- **Schedule edits can change derived historical expectations** → Persist the window, position, and scheduled-time snapshot when an outcome first resolves; continue to label unresolved legacy calculations as derived.
- **Legacy take allocation is imperfect after historical schedule edits** → Keep it deterministic and non-persistent, never claim an inferred explicit link, and avoid a speculative backfill.
- **Opaque keys can be tampered with** → Treat keys only as locators, re-derive and authorize every occurrence, and return PHI-safe not-found or validation errors.
- **Outcome/take races can double-resolve** → Lock the source, use unique identities, and keep take creation and outcome linking in one transaction.
- **A broad occurrence API could expose too much health history** → Require person-scoped authorization, enforce bounded ranges, paginate persisted history, and exclude clinical context from keys and logs.
- **The shared table contains a temporarily unsupported source type** → Reject direct-assignment writes until the dependent change enables them; the exact-source schema avoids a later unsafe migration.

## Migration Plan

1. Land `record-medication-pause-periods` and make its interval query available.
2. Add the occurrence table, exact-source and state constraints, foreign keys, unique indexes, and audit support without backfilling rows.
3. Deploy the derived occurrence query and resolver with formal schedule support only.
4. Switch dashboard, reminders, reports, history, and insights to the shared projection.
5. Add web, API, sync, portable-v2, capability, OpenAPI, release-note, and generated-client support.

Rollback may stop creating and reading explicit outcomes while retaining the additive table and audit evidence. Do not delete resolved outcomes or unlink immutable takes. Older code continues to interpret takes under its existing count-based behaviour; a forward fix must reconcile any outcomes recorded during the deployment window before readers are re-enabled.
