# Bounded parity rulings for #2347

Independent source review, 1 October 2026. These recommendations cover only the
compatibility disagreements assigned by the coordinator. They do not certify runtime
parity or resolve the other baseline failures. No source, tests or fixtures were
changed by the reviewer. The coordinator owns final contract decisions.

## People pagination: preserve the Rails clamp

Recommended ruling: an authenticated People collection request with
`per_page=999` returns 200 with `meta.per_page=100`. Keep the existing assertion
in [web_reads_api.rs](../../../../rust/contract-tests/tests/web_reads_api.rs),
line 334. Recommended defect labels: `rust`, `bug`.

The authoritative [OpenAPI](../../../api/openapi.v1.yaml), lines 252–279 and
4330–4347, defines a maximum page size of 100 but does not say an excessive
integer must produce 422. The observable Rails compatibility rule is explicit:
[PeopleController](../../../../app/controllers/api/v1/people_controller.rb),
line 6, uses shared `render_collection`; [BaseController](../../../../app/controllers/api/v1/base_controller.rb),
lines 178–180 and 225–227, clamps integer page sizes to 1–100 before querying.
[write_resources_spec.rb](../../../../spec/requests/api/v1/write_resources_spec.rb),
lines 32–41, asserts the same shared path returns 200 and size 100 for 500;
this is adjacent collection coverage, not a freshly executed People-999 proof.

Rust already implements the clamp in [response.rs](../../../../rust/api/src/read_resources/response.rs),
line 53, but People calls the stricter `parse_location_page` first in
[people.rs](../../../../rust/api/src/read_resources/people.rs), line 71;
[response.rs](../../../../rust/api/src/read_resources/response.rs), lines 63–68,
rejects the request before that clamp. Correct this bounded compatibility path
with actual RED/GREEN evidence. Do not infer changes to malformed values or
other resource contracts from this ruling. Clarifying the OpenAPI description
would make the accepted oversized-integer behaviour explicit.

## Eligible stock ordering: preserve location order

Recommended ruling: return scoped matching stock IDs in location-name ascending
order, then medication-ID ascending order. Preserve the location-move assertion
in [source_capabilities_api.rs](../../../../rust/contract-tests/tests/source_capabilities_api.rs),
lines 368–379. Recommended defect labels: `rust`, `bug`.

OpenAPI lines 7115–7126 and 7511–7522 describes this as an optional Rust browser
projection, and does not specify ordering. The Rails API serializers do not
publish this field: [schedule_serializer.rb](../../../../app/serializers/api/v1/schedule_serializer.rb)
and [person_medication_serializer.rb](../../../../app/serializers/api/v1/person_medication_serializer.rb)
publish their permission projection as `can_manage`. Therefore this is a browser
compatibility ruling supported by observable Rails selection behaviour, rather
than an invented existing Rails JSON ordering promise.

[MedicationStockSourceResolver](../../../../app/services/medication_stock_source_resolver.rb),
lines 73 and 100, orders both batch and individual matching inventory by
`locations.name ASC, medications.id ASC`. Rust instead sorts only IDs in
[source_stock.rs](../../../../rust/api/src/read_resources/source_stock.rs), line
115. This order reaches the browser unchanged through
[inventory.rs](../../../../rust/api/src/web_pages/inventory.rs), lines 118–172;
[medication.js](../../../../rust/web/src/medication.js), lines 29–38, appends
options in that order and selects the first ID. It therefore affects the visible
choice and default, not just a mathematically unordered set. Preserve all scope,
signature, supply and foreign-household filters when correcting ordering.

## Paused source: retain matching stock, reject recording separately

Recommended ruling: a paused origin can still project its scoped, matching,
available inventory. Its permission-derived `can_record` can remain true;
the pause is an independent prohibition on actually recording a dose. Correct
the empty-list expectation at
[source_capabilities_api.rs](../../../../rust/contract-tests/tests/source_capabilities_api.rs),
line 293, and retain explicit actual paused-submit rejection and no-write proof.
Recommended labels for this incorrect assertion: `testing` (Rust test scope).

[MedicationStockSourceResolver](../../../../app/services/medication_stock_source_resolver.rb),
lines 23–30, computes available medications by stock separately from
`blocked_reason`, which returns `:paused`. Its linked-stock queries, lines
80–82 and 142–144, also do not discard inactive alternate assignments. Do not
conflate a paused origin with an alternate stock source linked by an inactive
assignment. [RecordDose](../../../../app/services/medication_administration/record_dose.rb),
lines 125–132, checks `blocked_reason` before selecting stock. The Rails resolver
spec at [medication_stock_source_resolver_spec.rb](../../../../spec/services/medication_stock_source_resolver_spec.rb),
lines 83–89, independently asserts the pause block.

OpenAPI lines 7107–7126 and 7503–7522 separates dose-recording permission from
scoped matching stock, with submission rechecking both. Rust
[source_stock.rs](../../../../rust/api/src/read_resources/source_stock.rs),
lines 15–18 and 101–115, follows this separation, while
[dose.rs](../../../../rust/api/src/dose.rs), lines 874–879, rejects paused writes
with 422. The medication browser filters paused/inactive origins out before
offering dose controls in [inventory.rs](../../../../rust/api/src/web_pages/inventory.rs),
lines 102–109. Removing inventory IDs merely to satisfy the failing assertion
would introduce a new projection policy without Rails or OpenAPI support.

## Partly filled stock: show it, check the full dose when saving

The next run reached a further incorrect test expectation: stock of 1.00 ml
remained visible for a 1.25 ml dose. The authoritative
[OpenAPI](../../../api/openapi.v1.yaml), lines 7115 and 7511, defines the optional
stock projection as matching sources with untracked or positive supply. It also
requires submission to check stock again.

[MedicationStockSourceResolver](../../../../app/services/medication_stock_source_resolver.rb)
uses `out_of_stock?` when listing stock;
[SupplyLevel](../../../../app/models/supply_level.rb) defines this as a tracked
balance at or below zero. Rust checks the quantity needed for the full dose when
writing in [dose.rs](../../../../rust/api/src/dose.rs), for both medication stock
and a selected tracked option.

Correct the two insufficient-stock projection expectations in
[source_capabilities_api.rs](../../../../rust/contract-tests/tests/source_capabilities_api.rs).
Add actual dose submissions that require 422 and unchanged stock, dose history,
versions and sync records. Preserve the documented positive-stock projection.
The ordinary-stock assertion failed in the first run; the later tracked-stock
assertion had not yet been reached. Both corrected checks subsequently passed,
including rejected submissions and unchanged clinical records, in
`G-SOURCE-CAPABILITIES-CORRECTED-002`.

## Unknown dose source: return not found

A dose request with a valid source identifier and a supplied, nonblank string
source type that Rust does not support must return 404. Missing, blank,
whitespace-only or non-string source types, malformed identifiers and unknown
request fields retain their existing 422 responses. This applies to direct
recording and sync through their shared creation function.

[MedicationTakesController](../../../../app/controllers/api/v1/medication_takes_controller.rb)
raises `ActiveRecord::RecordNotFound` for an unknown source type. Its explicit
[request spec](../../../../spec/requests/api/v1/medication_takes_spec.rb) requires
404 for a valid request using `source_type: 'unknown'`.
[Sync::BatchesController](../../../../app/controllers/api/v1/sync/batches_controller.rb)
uses the same source lookup rule. No Rails changes are needed for this ruling.

`G-DOSE-SOURCE-ERRORS-RED-001` ran both new Rust tests against unchanged
production handling. Each passed eighteen malformed-input controls and one
unknown-field control, then failed because the supported request shape with an
unknown source type returned 422 rather than 404. All clinical records remained
unchanged. The existing `dose_write_api` expectation of 422 for this one case
therefore needed correction; its original six-case baseline passed and is
retained. The minimal shared validation fix subsequently passed both focused
tests and six route checks in `G-SOURCE-ERRORS-GREEN-001`. Independent review
verified the original matching source manifests and accepted this bounded fix.
The corrected six-case direct-write suite also passed. Its receipt retains the
pre-run manifest and matching emitted copied-source digest, but not the copied
per-file manifest; no stronger source-provenance claim is made for that run.
The wider dose checks subsequently passed as recorded below. Final composed
acceptance remains pending.

## Dose errors and audit records

Occurrence actions must record the occurrence controller as their audit source.
A paused occurrence returns the `paused` error code. When no matching source has
available stock, direct recording and occurrence recording report out of stock,
including when the submitted selection refers to an empty source. This check
comes before resolving a stock selection. Existing checks for malformed input,
access restrictions, retries and available stock selections keep their order.

Direct recording with a numeric JSON dose amount returns `validation_failed`,
the message `Validation failed`, and the field error `dose_amount: ["must be a
string"]`, matching Rails. Sync keeps its existing error contract. Invalid take
pagination remains rejected under the documented limits; a valid filtered page
has a separate passing check.

`G-DOSES-FIX-GREEN-001` passed all seventeen active dose tests and six route
checks. The existing timestamp-response test remains ignored. Independent
review accepted the error and audit fixes, the rejected-request checks and the
matching source manifests. The source-error and direct-write targets also
passed again on this source, with original matching manifests retained for
both runs. Full sync and retry targets passed twenty-three and eight tests
respectively. These results do not yet accept the proposed file splits or
replace final composed acceptance and publication.

## Schedule pagination and test corrections

The named `schedules` target failed the same eight tests before and after the
occurrence refactor: three passed and four existing cases remained ignored.
These results came from separate fixtures; the reconstructed before source
matched all 422 original application paths.

One failure is a Rust bug, tracked in
[#2367](https://github.com/damacus/med-tracker/issues/2367). Rails schedules use
the shared pagination helper, which converts malformed values to zero before
clamping the page and page size. The request `page=bogus&per_page=0` therefore
succeeds with page one and size one. Rust rejected it before its normalisation
could run. The reviewed correction accepts string pagination values only for
schedules, then normalises the proven malformed, zero, negative and oversized
cases after authorisation. Its signed decimal prefix parser is not a claim of
complete Ruby `to_i` equivalence. Other resources keep their existing parsing.

The remaining reached failures have reviewed test corrections: supply required
medication reorder thresholds, accept nullable pause-period fields, and validate
UTC timestamps without dropping microsecond precision. A successful full update
keeps the original person. A separate attempted person transfer must return 422
and preserve the saved body and version, as the API contract requires. The four
existing ignored cases remain unchanged.

The first repaired run passed eight tests and reached three more failures. A
new locations control incorrectly expected 400; that resource explicitly returns
422 for invalid pagination. The control now preserves that rejection.

The two numeric amount assertions found another Rust compatibility bug, tracked
in [#2368](https://github.com/damacus/med-tracker/issues/2368). Rails rejects a
numeric schedule amount with `dose_amount: ["must be a string"]`. The reviewed
follow-on preserves that assertion and adds only a numeric-value rejection at
the existing schedule validation boundary. Other decimal errors and validation
priorities stay unchanged.

The corrections now cover three authored files. They are installed and
independently reviewed; the first correction passed compiler and lint checks.
The repaired reconstructed before version passed all eleven active schedule
tests and six route checks; its four existing ignored cases remain unchanged.
Independent review accepted its source and behavioural evidence. The matching
run on the current code also passed eleven active tests and six route checks.
Its source and fixture hashes are runner-emitted only; no independent original
copy or fixture manifest is claimed. Independent review accepted the occurrence
checkpoint with that qualification. Publication and final combined checks remain.
Rails behaviour here was verified from current source, not live Rails requests.

## Verification limit

The original reviewer derived these rulings from source and did not execute
Rails requests. The coordinator subsequently accepted the Rust checks recorded
in [verification-queue.md](verification-queue.md):
`G-WEB-READS-PAGINATION-GREEN-001` covers oversized People pages and malformed
query controls; `G-SOURCE-CAPABILITIES-CORRECTED-002` covers location ordering,
paused submissions and insufficient-stock submissions. Both receipts retain
matching source manifests and unchanged-record checks. These changes remain
local until the final tranche is published and its required CI passes.
