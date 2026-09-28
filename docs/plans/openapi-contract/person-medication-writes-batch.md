# Person medication assignment writes

Next medication batch after administration settings: createPersonMedication,
updatePersonMedication and replacePersonMedication. Verify related existing
assignment GET list/detail behaviour alongside the writes where practical;
keep the original 89 baseline separate from full 118 completion.

Use current OpenAPI plus PersonMedicationsController, PersonMedication and
PersonMedicationPolicy for omitted semantics. Existing Rust assignment
serializer, read entities and management helpers are reusable. The older
assignments.rs tests are references, not credited Rust runtime evidence.

POST requires person_id and medication_id, returns 201 and ETag, and handles
optional dosage option, amount/unit, administration kind, notes, max daily
doses, minimum interval and dose cycle. Verify managed person and medication
visibility, numeric/portable links, duplicate active assignments, position,
default dose and dosage-option snapshot/matching. PATCH and PUT merge supplied
fields. If-Match is optional; stale values return 409, not missing-header 428.
Invalid writes preserve the existing resource and ETag. Verify audit/sync
effects and shared idempotency wiring from the preceding batch.

Do not reproduce the known Rails 2.125-to-2.13 silent rounding defect. Apply
the already-established dosage exact-storage-precision rule to this decimal
field, reject unrepresentable values with 422, and document its scale before
deriving tests. Preserve documented compatibility for other fields; report
unresolved policy conflicts rather than inventing rules.

Sol owns production/specification/shared tooling and Luna owns a separate
focused test file. Initial compiled RED precedes implementation; expand tests
in parallel, independently review before final isolated acceptance, then
record evidence and publish. Pause/resume/reorder and pause-history operations
remain subsequent medication batches.
