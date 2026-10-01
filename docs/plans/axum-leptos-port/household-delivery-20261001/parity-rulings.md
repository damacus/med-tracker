# Bounded parity rulings for #2347

Independent source review, 1 October 2026. These recommendations cover only the
three disagreements assigned by the coordinator. They do not certify runtime
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

## Verification limit

All three rulings above are source-derived recommendations. No runtime command,
Rails request, fixture mutation or screenshot was executed by this reviewer.
Acceptance requires the exclusive verifier's receipts for the matching source,
including strict pagination, location ordering and paused-write no-write checks.
