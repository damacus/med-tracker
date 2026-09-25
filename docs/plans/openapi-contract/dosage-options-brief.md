# Dosage option contract tranche

Source: `docs/api/openapi.v1.yaml`, paths `/households/{household_id}/dosage_options` and `/households/{household_id}/dosage_options/{id}` (operations `listDosageOptions`, `createDosageOption`, `getDosageOption`, `updateDosageOption`, `replaceDosageOption`). Base is `863585e0`; the Rust router has no dosage-option routes at that base.

Test first: exercise the five routes with an authorised fixture; assert exact request and response fields, decimal strings, numeric and portable IDs, pagination and timestamp filters, malformed and invalid input envelopes, ETags, stale write preconditions, and household scoping. Run all tests once against absent routes for RED, then implement SeaORM handlers and rerun in the isolated Compose project. Record assertions and remaining gaps per operation in the coverage map.

OpenAPI does not yet define dosage-option role/visibility rules. The `If-Match` header is optional; an absent header allows an update, while a stale supplied value returns `409`. Role and visibility cases remain provisional until authority is clarified. Database columns permit null values for several fields the response schema requires; do not invent representation defaults. Characterise any existing invalid rows separately from contract-created rows.

Storage has `numeric(10,2)` for supply/reorder and `numeric(4,1)` for minimum hours. Input decimal strings are parsed exactly and values outside representable precision/range are rejected with the documented `422` rather than rounded by PostgreSQL. The OpenAPI decimal schema itself has no corresponding precision limits; this remains a contract/storage gap for broader policy review.
