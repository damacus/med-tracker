# Dosage option contract tranche

Source: `docs/api/openapi.v1.yaml`, paths `/households/{household_id}/dosage_options` and `/households/{household_id}/dosage_options/{id}` (operations `listDosageOptions`, `createDosageOption`, `getDosageOption`, `updateDosageOption`, `replaceDosageOption`). Base is `863585e0`; the Rust router has no dosage-option routes at that base.

Test first: exercise the five routes with an authorised fixture; assert exact request and response fields, decimal strings, numeric and portable IDs, pagination and timestamp filters, malformed and invalid input envelopes, ETags, stale write preconditions, and household scoping. Run all tests once against absent routes for RED, then implement SeaORM handlers and rerun in the isolated Compose project. Record assertions and remaining gaps per operation in the coverage map.

At the first checkpoint, OpenAPI did not yet define dosage-option role/visibility rules. The completion tranche now documents the existing Rails controller and model rules. The `If-Match` header is optional; an absent header allows an update, while a stale supplied value returns `409`. Database columns permit null values for several fields the response schema requires; do not invent representation defaults. Characterise any existing invalid rows separately from contract-created rows.

Storage has `numeric(10,2)` for supply/reorder and `numeric(4,1)` for minimum hours. Input decimal strings are parsed exactly and values outside representable precision/range are rejected with the documented `422` rather than rounded by PostgreSQL. The OpenAPI decimal schema itself has no corresponding precision limits; this remains a contract/storage gap for broader policy review.

## Completion tranche after `1467df53`

The five route implementations existed, but policy, inventory callbacks, and sync events were incomplete. Existing Rails controller, model and policy rules supplied the missing semantics. OpenAPI now states active-member reads scoped to visible medications; household owner/administrator writes; positive dosage and nonnegative inventory values; default uniqueness; parent single-dose clearing and inventory synchronization. The list uses the same medication visibility as detail even though the Rails list scope is broader. Tests first ran against the existing Rust implementation and failed on policy, validation, parent state and event behaviour. Follow-up tests cover multiple tracked options and unique-default rollback; the consolidated acceptance passed 11/11.

The nullable historical database fields conflict with the current nonnull response schema. A user decision on the response contract is pending; no substitute values or silent filtering will be introduced. All other cases can be implemented independently. Parent and option changes must commit atomically, with household lock before parent medication lock before dosage lock, and parent and option sync events sharing the HTTP request ID.
