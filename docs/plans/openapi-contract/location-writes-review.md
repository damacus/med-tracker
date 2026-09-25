# Location write tranche review

## Scope and requirements

Reviewed the location POST, PATCH, PUT and DELETE handlers, membership POST
and DELETE handlers, read-test additions, and isolated acceptance dispatch.
OpenAPI remains the source of requirements. No Rails application or Leptos
changes are included.

The tested owner path includes strict JSON attributes, null and absent
description handling, numeric and portable identifiers, version headers,
atomic concurrent updates, membership replay and household isolation.
Seven independently executing write tests initially failed on missing
routes. The first combined green run passed eight write and four read tests.

The initial pagination test could pass if page two repeated page one. The
writer strengthened it to compare IDs and total counts. Review also found
that generic UUID parsing accepted forms outside the documented pattern;
the handler now validates the exact identifier shape.

Four subsequent tests demonstrated malformed and overflowing numeric query
values returning framework 400 responses instead of documented JSON 422.
The location-only fix authenticates before converting query rejection to
the audited JSON 422 response. Final runtime evidence confirms eight write
and eight read tests pass, including all four malformed-pagination cases.
The isolated project cleaned up successfully; the reviewed log is
`1790366360_task_api_7d11fd.log`.

## Code review

SeaORM handles ordinary reads and writes. Update and delete obtain a row
lock before checking the current representation ETag. Membership insertion
and deletion lock the location, serialising assignment replay through these
handlers. Failed constraints use savepoints where needed to preserve the
outer audit transaction. Request values are not interpolated into SQL.

Fixture SQL is confined to isolated test data and verification. Successful
deletion tests assert an empty HTTP response body. Cross-household writes
check that the foreign record remains unchanged.

## Unresolved requirements

This is not approval of complete location API coverage. The owner-only write
guard is interim. OpenAPI does not define location visibility, managed-person
eligibility, the full deletion-retention rule, or rate-limit thresholds.
The user has been asked whether to preserve and document existing MedTracker
rules. Those answers remain pending. No full-role or retention-parity claim
is supported by the current tests.

Code may be retained as an explicitly partial checkpoint; the tranche must
not be marked complete or moved to UI work while these requirements remain
unresolved.

Code-quality verdict: no outstanding blocking finding in the reviewed
implementation. Requirements verdict: partial only, pending the policy
decisions and corresponding tests above.
