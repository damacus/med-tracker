# Profile and avatar contract tranche

Cover the six fixed operations `getHouseholdProfile`, `updateHouseholdProfile`,
`updateHouseholdProfileWithPut`, `downloadCurrentAvatar`, `uploadCurrentAvatar`,
and `deleteCurrentAvatar`. `getCurrentProfile` is already covered by
`openapi_people.rs` and is outside this tranche. No Rust profile/avatar routes
or handlers were found in `rust/api/src` at this checkout; do not count these
operations until focused HTTP acceptance passes against Rust.

Existing Rails-backed evidence is in `rust/contract-tests/tests/profile.rs`:
profile self/foreign/grant access, current grant revocation, PATCH/PUT updates
and rollback on invalid input, private PNG/JPEG/WebP upload/download, declared
MIME and Content-Disposition headers, no-store, exact 5 MiB boundary and
oversize rejection, replacement preservation, 204 delete state, and upload /
remove audit event types. Reuse those broad scenarios rather than duplicating
them.

`rust/contract-tests/tests/openapi_profile.rs` adds exact seven-field profile
shape/type checks, empty and partial PATCH/PUT behavior, strict request fields,
required profile wrapper, invalid-input nonmutation, the required multipart
avatar part, response metadata, post-delete attachment absence and 404
download, and request-correlated removal audit metadata. The smallest selected
runtime slice is the new `openapi_profile` target plus existing `profile`;
`openapi_people` remains separate proof for `getCurrentProfile`. OpenAPI does
not declare 429 for this family and explicitly says avatar multipart writes
are not response-cached by Idempotency-Key, so no such assertions are added.

Storage/security limits: Rails validates the ActiveStorage blob MIME label and
size; existing fixtures upload arbitrary bytes labeled as images, so these
tests prove declared MIME and byte transport, not image decoding or signature
validation. Deletion tests prove the attachment is detached and inaccessible
through the API. An isolated API with an unwritable storage root now proves
download and replacement return 503 without replacing the existing attachment
or bytes. Physical cleanup after database detachment can leave orphaned bytes
if deletion fails; the suite does not claim guaranteed physical purge.

The avatar methods inherit broad 400/422/503 response references in OpenAPI.
GET has no request-body validation and DELETE has no renderer or upload, so
ordinary GET 400/422 and DELETE 400/422/503 are not manufactured or claimed as
tested. The method-specific failure paths are recorded in the evidence ledger.

JSON profile PATCH and PUT also reuse Rails' optional idempotency behaviour:
keyed retries replay without duplicate person versions or sync events, while
changed payloads conflict. Multipart upload remains excluded from response
caching. Profile writes reload current authority and serialise account
preference updates; supplied date-of-birth changes create a correlated Person
version and sync event.

The profile response schema requires a non-null date of birth, matching the
Person model validation; keep exact response checks on contract-created
fixtures. Independent review and isolated acceptance are required before
ledger credit.

Disk storage uses an absolute `ACTIVE_STORAGE_ROOT` (default `/app/storage`)
and `ACTIVE_STORAGE_SERVICE_NAME`, falling back to the existing Rails
`ACTIVE_STORAGE_SERVICE` setting (`test`, `local`, or `persistent`; default
`persistent`). Unsupported services fail at startup. Existing ActiveStorage keys retain their two-level directory
layout. A blob from another service is not read through the configured disk
root. Object-storage providers are not implemented by this disk adapter.
