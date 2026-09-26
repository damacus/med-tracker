# Web push subscription batch

User authorised the next low-risk tranche under the same budget rules.
Starting usage is 96%, leaving four percentage points; preserve capacity for
verification and publication. No reset credit is authorised.

Scope: `createPushSubscription` and `deletePushSubscription`, both absent
operations in the original 89-operation baseline. Sending a test notification
is outside this tranche. Registration and revocation make no network requests.

## Acceptance and ownership

Preserve account-owned upsert, global endpoint uniqueness, empty 201/204
responses, idempotent revocation and query-string endpoint decoding. Reject
malformed wrappers, missing query parameters, invalid keys and unsupported
endpoints without changing state. Preserve the existing HTTPS provider
allowlist, including proper subdomain boundaries and rejection of credentials
and IP addresses. Verify concurrent claims have one owner; foreign-account
revocation cannot remove that owner's record. Keep endpoint and push keys out
of errors and audit metadata. Reuse shared authentication and limiter proofs
with direct endpoint assertions.

Sol owns production, OpenAPI clarifications and runner wiring. Luna owns the
new HTTP test file. Demonstrate RED before implementation, compile tests before
isolated builds, review before final acceptance, and avoid redundant broad
checks. The orchestrator owns independent review, the ledger and publication.
No dependencies, schema changes, external push delivery or additional API
family belongs in this tranche.

## Initial evidence

The compiled test suite ran before production changes in isolated project
`mtcontract-f5a5ae7352a24cbe`: four groups failed on missing routes returning
404, and shared rate-limit wiring passed. Log
`/Users/damacus/Library/Application Support/rtk/tee/1790423799_task_api_694219.log`.
The project was cleaned up. One test initially read a success header before
checking status; review reordered those assertions to report failures clearly.
The final test file compiles and passes formatting checks before acceptance.

## Completed

Final isolated HTTP acceptance passed all five groups in project
`mtcontract-48617928f63f4ae9`, which was cleaned up. Evidence:
`/Users/damacus/Library/Application Support/rtk/tee/1790424187_task_api_694219.log`.
Independent requirements/code review passed, including account locks,
savepoint recovery on global uniqueness conflicts, URL allowlist boundaries,
unchanged stored endpoints, query decoding and secret-free audit records.

API check, Clippy, all 15 unit tests, formatting, selected contract compilation
and runner dispatch/failure-cleanup checks passed. No Rails code, database
schema or dependencies changed. Both operations are fully verified against
the bounded documented contract; the original baseline now has 19 complete
and 70 remaining. Reported allowance moved from 96% to 97% used before
publication. No reset credit was consumed.
