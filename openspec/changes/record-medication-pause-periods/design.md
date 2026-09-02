## Context

`Schedule` and `PersonMedication` currently include `Pausable`, which flips an `active` boolean. Web and API actions call `pause!` or `resume!` directly, so the application cannot represent multiple pause periods or explain a pause after resume. Reporting derives expectations from current source state and cannot exclude historical pause intervals.

The medication-administration context owns both source lifecycles. Interoperability adapts that state for API, sync, and portable data; it must not become a second pause implementation. `/api/v1` cannot make the existing optional pause body mandatory without a new major API version.

## Goals / Non-Goals

**Goals:**

- Use one lifecycle contract for formal schedules and direct assignments.
- Preserve current `active` behaviour while adding attributable interval history.
- Make pause/resume atomic, tenant-safe, auditable, idempotent, and usable by report queries.
- Add context-required API support without breaking existing `/api/v1` clients.

**Non-Goals:**

- Trigger reorder, restock, or stock-removal workflows from a reason.
- Decide whether a medication should be paused.
- Reconstruct pause history that the database never recorded.
- Replace source retirement or medication-take history.

## Decisions

### Store pause periods as administration records

Add a household-scoped `MedicationPausePeriod` with nullable `schedule_id` and `person_medication_id` foreign keys plus a database check that exactly one is present. This follows ADR 0006 and retains concrete household-safe foreign keys instead of a polymorphic association.

The record stores a portable ID, reason string, optional note, nullable `started_at`, nullable `ended_at`, recording membership, resuming membership, timestamps, and a legacy-context flag. New records require `started_at` and a public reason. Legacy backfills may use `reason_not_recorded` and a null start only when the legacy flag is true.

Use two partial unique indexes to permit at most one open period for each source type. Check constraints reject an end before its known start and reject legacy-only values on new records. PaperTrail versions the period; existing audit-ledger triggers preserve the attributable change evidence.

Rejected alternatives:

- Columns on each source would retain only the latest pause and duplicate lifecycle rules.
- PaperTrail-only history cannot provide a stable, portable domain timeline or reliable report query.
- A polymorphic source would weaken foreign-key and tenant integrity.

### Move pause and resume into one transactional boundary

Introduce one medication-administration service for pause and one for resume. Each service resolves the concrete source, authorizes before invocation, locks the source, and changes the source plus its period inside one transaction using bang persistence.

Pausing an already paused source returns its current open period without duplicating it. Resuming closes the open period and activates the source. A source that is inactive with no period receives a legacy-context period before it can resume. Database uniqueness handles concurrent callers; the service reloads the winning period for an idempotent response.

Controllers remain delivery adapters. The model concern delegates to this boundary or is reduced to state predicates so new callers cannot bypass period creation.

### Calculate expectations from interval state

Current/future queries continue to use `active` for efficient filtering. Historical and range-based expectation queries preload pause periods and exclude only the overlapping interval. An unknown legacy start excludes expectations from the migration timestamp onwards; it does not change earlier report results.

Do not query inside components or loops. Controllers and report/query objects preload the current or range-relevant periods and pass the result to presenters.

### Add a compatible API transition

Add a capability-advertised pause-period create operation that accepts `source_type`, portable `source_id`, reason, and note. Add a resume operation against the open period. Both use normal API idempotency and error envelopes.

Keep the existing schedule and person-medication pause/resume operations for at least the documented deprecation period. A legacy pause creates `reason_not_recorded`; its response adds current pause context without removing existing fields. Mark the operations deprecated in OpenAPI and release notes. Generated clients use the new operation after capability discovery.

Sync snapshots and change feeds include pause-period records. Sync batches support creating and closing a period through the same service and ETag rules. Portable v2 includes the new collection; portable v1 remains unchanged. Import restores periods transactionally without replaying lifecycle side effects and reconciles each source's final `active` state.

### Keep UI context small and accessible

Replace immediate Pause links with a RubyUI dialog containing a required reason select, optional note, validation summary, Cancel, and Pause actions. Resume remains direct but shows the context being closed. Cards show the active reason; the existing medication history surface shows completed periods.

The dialog uses existing focus trapping, keyboard dismissal, labels, error associations, Turbo replacement, and responsive component patterns. Reason copy records what the user chose and never recommends pausing.

## Risks / Trade-offs

- **Legacy pauses have incomplete history** → Label them explicitly, keep the start unknown, and avoid changing historical calculations before migration.
- **`active` and an open period can diverge through old code** → Route every writer through the service, retain database constraints, and add a consistency check used by migration and verification.
- **Deprecation temporarily permits context-free API pauses** → Advertise the new capability, update first-party clients immediately, record `reason_not_recorded`, and remove the old shape only under the API versioning policy.
- **Range reports could introduce N+1 queries** → Fetch relevant periods at the query boundary and verify query counts only where the existing performance spec covers the projection.
- **Concurrent pause/resume calls can interleave** → Lock the source and enforce one open period with partial unique indexes.

## Migration Plan

1. Add the pause-period table, exact-source constraint, household foreign keys, time checks, and open-period indexes.
2. Backfill one open legacy period for each inactive current source with unknown start and `reason_not_recorded`; do not backfill active sources or completed history.
3. Deploy services and readers while retaining `active` as the compatibility state.
4. Add web, API, sync, portable-v2, OpenAPI, capability, release-note, and generated-client support.
5. Verify every inactive source has one open period and every active source has none before treating period state as authoritative for historical calculations.

Rollback may remove readers and new operations while leaving the additive table in place. Do not drop recorded periods during rollback. Continue deriving current state from `active` until a later separately approved cleanup removes that compatibility field.
