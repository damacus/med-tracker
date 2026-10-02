# Medication management API product handoff

## Frozen candidate

The management candidate was copied from eight API paths before the additive onboarding work resumed: `rust/api/src/audit.rs`, `dose.rs`, `entities.rs`, `lib.rs`, `medication_management.rs`, `stock_removals.rs`, `audit_logs.rs`, and `sync_events.rs`.

It adds household-scoped medication create, PATCH/PUT, stock adjustment and reorder actions, stock-removal history and idempotent removal, and manager-only audit history. Conditional writes compare the current Rust representation ETag while holding the medication row lock. Successful writes produce an attributable request audit and field-shaped domain versions. Tracked removal updates the selected dosage and the medication supply, threshold, and last-restock baseline together. A switch from option mode to a single dose clears linked PersonMedication option references, deletes options, and records dosage tombstones and affected source changes; a null-to-null patch has no domain version.

The later sync correction records Rails-shaped Medication, MedicationDosageOption, and MedicationTake change events on actual mutations, with account, membership, request ID, portable ID, and person portable ID for takes. Mutations lock household before medication and dosage rows so the sync cursor barrier precedes event timestamps. Replays and rejected writes create no domain change events.

## Evidence and boundary

The first management RED had all nine medication-stock cases failing at absent POST handling (405). A diagnostic candidate then passed 66/67; the remaining same-ETag contention case exposed a missing Medication `api_update` domain version. Focused dose-mode RED was 0/2: null-to-null returned 422 and a valid switch left dosage rows. Focused sync RED was 0/3: Medication create event absent, MedicationTake metadata empty, and tracked removal option event absent. These failures drove the bounded fixes.

Before the eight-file snapshot, `rtk task api:fmt:write`, `rtk task api:clippy`, and `rtk task api:test` passed (10/10 unit tests); scoped `git diff --check` passed. Independent source review found no remaining blocker in the transition or sync-event lock order. The immutable candidate's canonical 72-case HTTP result is pending in this report; local static checks alone do not establish runtime acceptance.

This handoff covers the named medication management contract and focused security, transition, and sync cases. Composite wizard onboarding, location reads, and additive refill are a separate in-progress tranche. Browser journey acceptance is separate from these API checks. Other Rails medication actions and full Rails ETag byte identity are outside this contract.
