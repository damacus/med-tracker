# Medication management API: independent review

Scope: the existing nine `medication_stock.rs` contracts selected after the
accepted 53 HTTP cases. This review covers medication create/PATCH/PUT,
inventory adjustment and reorder transitions, stock removal/history, and the
household security-audit list. It does not expand the browser journey or
scheduled-dose actions.

## Early requirements findings

Rails `MedicationsController` and `MedicationPolicy` separate permissions by
action. Current household managers may update, adjust, and remove stock. A
member with a current person `manage` grant may create medication in a scoped
location. A member with `view` access to a visible medication may mark its
reorder status, while a hidden medication stays 404. The nine contracts prove
the main owner, viewer, hidden, foreign, and delegated-create paths; the Rust
implementation must check these permissions on each request, including replay.

Medication create and replacement must apply the model's permitted-field
validations beyond the few values in these nine tests: required name/location
and reorder threshold; positive dose amount; allowed dose unit/category;
nonnegative supply; and barcode shape/uniqueness. A transition from dosage
options to a single standard dose is forbidden while schedules use those
options; otherwise Rails synchronizes option deletion and assignment source
references. Silent partial mode conversion would corrupt later dose selection.

Rails `RemoveMedicationStockService` locks the selected dosage and medication
before checking a submission replay or available stock. A successful removal,
dosage decrement, parent medication aggregate sync, and
`MedicationStockRemoval` version form one transaction. An identical
`submission_id` returns the original event without another decrement;
changed quantity, reason, note, or dosage is rejected without a partial write.
The Rails request and concurrency specs also require tracked dosage selection,
foreign dosage denial, and one winner for simultaneous duplicate or competing
removals. The selected nine HTTP tests currently exercise only ordinary
sequential stock removal. The test owner has frozen five supplemental HTTP/DB
cases for tracked dosage, concurrent removals, current-role replay, concurrent
conditional PATCH, and manager-only audit listing. I corrected their initial
PaperTrail `update` filter to Rails `api_update` and asked for realistic
tracked-mode setup; both corrections are present. Product owner received the
lock/transaction requirements.

Conditional medication PATCH/PUT accepts an absent `If-Match` in current Rails
behaviour and returns 409 for a stale supplied tag. The tag represents the
record class, ID, and `updated_at` timestamp in `Api::RecordEtag`; the OpenAPI
description of a serialized/derived-field hash does not match that source.
The supplied tag should be compared against the current value under the update lock so two
concurrent requests with the same old tag cannot both commit. The selected
test covers sequential stale writes, not this race.

The audit endpoint must return the newest 100 events from the requested
household before recording its own GET request event. Each accepted, rejected,
and replayed authenticated write needs a fresh attributed `api.request` event
without leaking clinical fields through generic database errors. Rails
`AuditLogPolicy#index?` and its scope require a household manager; the web
admin controller enforces that rule and policy specs allow owner and
administrator while denying an ordinary member. The API
`Admin::AuditLogsController#index` omits authorization and returns actor
account/membership IDs, request IDs, and metadata to any active household
member. Its managers-only request spec has only a manager positive case.
This is a confirmed Rails API authorization defect to correct in Rust with a
current-manager check; the supplemental test denies ordinary and delegated
members and checks a promoted administrator. The selected nine only read as
the owner.

## Status

Requirements findings were sent to product, test, and orchestration owners
before production edits. The nine original contracts and five supplemental
security cases were reviewed; two further transition cases now distinguish a
real single-dose switch from a null no-op. The first compiling Rust source had
six concrete issues:

1. Stock removal locks dosage before medication; the existing dose path locks
   medication before dosage. Competing requests can deadlock unless both paths
   use the same order and re-read under lock.
2. New error responses put a request ID in the header/audit but omit the same
   ID from `error.request_id` in the JSON envelope.
3. Tracked dosage removal updates the parent current supply but omits Rails'
   synchronized reorder threshold and supply-at-last-restock calculation.
4. Medication mutations still need atomic attributable domain versions;
   the product owner already identified this missing work.
5. Stock-removal submission UUID parsing accepts forms outside the Rails
   lower-case hyphenated 8-4-4-4-12 format.
6. Unbounded numeric(10,2) input can reach PostgreSQL as an overflow and
   produce a server error instead of a validation response.

Current source resolves all six: medication then dosage lock order, matching
error-envelope request IDs, tracked inventory synchronization, atomic domain
versions, strict UUID shape, and bounded numeric stock values. Domain version
changes now use per-field `[old,new]` pairs. Subsequent fixes bound reorder
quantity, validate the effective DMD code/system after PATCH, and use a nested
transaction to map a globally duplicated barcode to generic 422 while keeping
the outer request audit. The nested rollback is necessary because PostgreSQL
otherwise leaves the transaction aborted; unrelated database failures remain
server errors. The audit list's current-manager check, household filter and
snapshot before its own request audit appear correct in this source pass.

Two further source findings were resolved before acceptance. Rails blocks only a
blank-to-present single-dose transition when any schedule exists; a supplied
`dose_amount: null` on an existing option-mode medication is a no-op. The
test owner established a focused 0/2 RED; current Rust source now gates the
schedule check on a present new amount and returns the unchanged resource for
the null-only request. Rails `Medication` and
`MedicationDosageOption` include `SyncTrackable`.
Actual create/update/stock/reorder writes emit `api_change_events` consumed by
the native sync feed. The first Rust management source wrote domain versions
but omitted Medication events; tracked option stock removal omitted its option
event. The product owner added events for actual mutations only, with no
domain event for an identical replay or null no-op.
`Api::ChangeRecorder` locks the household before assigning event time, so the
new shared writer must preserve the household cursor barrier and a consistent
household, medication, dosage lock order. The accepted direct-dose path also
needed its parent medication and selected option events, and its take event
needed `person_portable_id` metadata for nonmanager sync visibility. Root
expanded the focused regression/fix to this existing path. The shared writer
now records Rails-shaped metadata after the household lock; management stock
removal uses household, medication, dosage lock order, and direct dose already
acquired those locks in that order.

A later review found that tracked stock removal changed the parent's
current supply, reorder threshold, and last-restock baseline but initially
versioned only supply. The product owner now includes all three before/after
fields in the same attributable version; that finding is resolved.

## Accepted management API verdict

**Requirements: PASS for the bounded medication-management API tranche.** The
frozen candidate at `/tmp/medtracker-management-candidate-20260925` passed
72/72 canonical HTTP contracts, including the original nine management
cases, five security/concurrency cases, two dose-mode transition cases, and
three sync-event cases. The same run retained 7/7 login and 7/7 previously
accepted medication browser journeys. Its 1,581-path runtime manifest matched
before and after the run, and cleanup completed. These browser cases establish
regression stability; the new management HTML UI has separate pending browser
acceptance.

**Code quality: PASS for the reviewed eight-file API candidate.** Independent
source review found current permission checks, tenant scoping, conditional
write locking, stock and audit transactions, bounded decimal handling, exact
submission replay, one-time domain effects, and the household sync cursor
barrier consistent with the bounded Rails authority. The audit list is
deliberately stricter than the Rails API route because Rails omits its own
manager-only policy check. The ETag uses Rust's existing representation hash,
which is internally consistent and follows the OpenAPI description, while
Rails currently hashes class, ID and timestamp. These are documented contract
differences, not failed management acceptance. Composite onboarding/refill,
their first-party UI, and full CRUD parity remain separate work.

## Bounded first-party management UI review

The new HTML wizard, edit form, refill dialog and signed Save receipt call the
shared API through the existing browser session. Current source checks the
configured request origin and form CSRF before forwarding writes; the API
rechecks authorization and domain rules. A Save receipt is bound to purpose,
current session, household, medication, source and a short lifetime. Its GET
also reloads the current medication and source through authorized API reads.
The form categories match the Rails medication list, schedule inputs produce
choice-specific configuration, and refill 422/409/403 responses preserve the
entered quantity, date, key, alert and HTTP status when access remains valid.

The first edit-conflict implementation reloaded a fresh ETag on 409 while
retaining stale submitted values, allowing a second Save to overwrite a
concurrent writer. Product now retains the submitted ETag on 409, provides a
clear conflict message and an explicit reload link. A second Save stays stale
until the user reloads current values. The focused unit test passed after a
RED; the independent browser regression and combined UI runtime remain
pending. Current bounded UI source has no other blocking finding.
