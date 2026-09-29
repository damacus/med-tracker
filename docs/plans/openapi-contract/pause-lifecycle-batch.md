# Medication pause history and ordering

After assignment and schedule writes, implement these eight operations:
listMedicationPausePeriods, createMedicationPausePeriod,
resumeMedicationPausePeriod, pauseSchedule, resumeSchedule,
pausePersonMedication, resumePersonMedication and reorderPersonMedication.

Reuse the Rails controllers, pause/resume services, serializers and existing
assignments.rs and schedules.rs scenarios. Those tests currently exercise the
Rails reference; they are not evidence that Rust implements the routes.

Require current person management permission for every mutation, including
owners and administrators. Preserve read visibility, cross-household isolation,
retired-source history, portable pause identifiers, strict envelopes and
pagination. Pause creation records server time, reason, note and actor. Repeated
pauses retain the existing open period. Addressed resume must not close a newer
period when an old completed period is retried. Preserve reason and recording
actor; record resume time and actor. Verify stale ETag, fresh replay authority,
idempotency, correlated request audits, transactional history and concurrency.

Legacy pause endpoints deliberately create reason_not_recorded context. This
is documented historical context, not permission to create incomplete doses.
Reorder swaps adjacent positions for the same person atomically; an edge move
is unchanged. Reject directions other than up/down with 422 rather than copying
Rails' silent success for invalid directions.

The pause-create prose says client timestamps are ignored, but its strict
request schema forbids them. Follow the strict schema: reject supplied
timestamps and other unknown fields with 422, preserving server-owned times.
Correct that prose explicitly; this is a documented compatibility correction,
not a new operation or an expansion of the fixed 118-operation scope.

Luna owns openapi_pause_lifecycle.rs. Sol owns a separate pause_lifecycle.rs
module; primary Sol retains shared router/entity/runner ownership. Tests must
compile and demonstrate missing-route RED before production is written. Use
owned source records to avoid sequential fixture pollution. Independent review,
isolated acceptance and publication are required before completion credit.
