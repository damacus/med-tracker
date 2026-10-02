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

The person association is immutable on update. Supplying the current numeric
or portable person identifier is accepted; a different visible person returns
422, while hidden or foreign identifiers retain the opaque 404 boundary.
Document this explicitly instead of copying Rails' silent ignored reassignment.
Existing medical history must not be moved between people by an ordinary edit.

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

Assignment and schedule modules have separate production/test owners and one
shared router/entity/runner owner. Integrate and compile both in parallel;
publish the two reviewed families together after both isolated acceptance
targets pass. This avoids making schedule verification wait for an intermediate
commit while preserving independent evidence for each operation.

The existing Rust web_pages consumer reads undocumented can_record and
eligible_stock_medication_ids fields from assignment/schedule API responses.
Strict OpenAPI responses exclude those fields, matching the Rails API
serializer. This exposes a deferred UI integration dependency: the paused
web port must obtain permission and stock choices through an agreed API
contract before its dose-entry flow can be credited complete. The original
paused UI checkout remains untouched; this batch does not verify UI parity.

Updates that leave all values unchanged preserve the resource timestamp and ETag,
and do not create version or sync-change records. Request auditing and keyed
response replay still apply. This corrects spurious Rails sync callbacks on
unchanged saves rather than claiming literal callback parity.
