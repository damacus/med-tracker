## Why

The generic API idempotency store persists response status and JSON body but drops response headers, so an original successful mutation can include an `ETag` while a later replay of that same response does not. This breaks the optimistic-concurrency contract used by iOS and generated API clients and blocks reliable mobile retries.

Originating issue: https://github.com/damacus/med-tracker/issues/1749

## What Changes

- Persist only an explicit, non-sensitive allowlist of response headers for completed idempotent mutations, beginning with `ETag`.
- Restore the stored allowlisted headers before rendering a replayed response.
- Add a compatible JSONB column whose empty default keeps existing idempotency rows readable without inventing unavailable historical header values.
- Enforce the existing 24-hour idempotency expiry under the serialized reservation boundary so legacy headerless rows age out safely without reopening a fresh key concurrently.
- Prove original and replayed create/update responses have matching ETag behavior while preserving account, household, conflict, rollback, and concurrent-serialization guarantees.
- Keep `Retry-After`, rate-limit metadata, cookies, authentication headers, request identifiers, and every other non-allowlisted header out of stored replay data.

Explicit non-goals:

- General-purpose response caching or arbitrary header persistence.
- Reconstructing historical ETags that were never stored.
- Changing endpoint-specific idempotency such as medication-take `client_uuid`.
- Changing the public response body, status, authorization, household isolation, or mutation semantics established by #1747.
- Adding a background cleanup worker or changing the 24-hour retention period.

## Capabilities

### New Capabilities

- `api-idempotent-response-headers`: Idempotent API mutation replays preserve explicitly allowlisted response headers and safely age out legacy headerless rows.

### Modified Capabilities

None.

## Impact

- Adds one compatible PostgreSQL JSONB column to `api_idempotency_keys`.
- Extends `Api::IdempotencyStore` and `Api::V1::BaseController` without changing endpoint response schemas.
- Adds focused service, migration, and API request coverage, including regression coverage around the serialized reservation boundary.
- Unblocks iOS and generated API clients that require the original ETag after a lost-response retry.
