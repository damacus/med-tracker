## Why

The generic API idempotency middleware checks for a stored response before a mutation and stores the response afterward, so concurrent requests can both execute before either result exists. Native clients therefore cannot safely retry People or other API writes without risking duplicate records and mutation side effects.

Origin: [GitHub issue #1747](https://github.com/damacus/med-tracker/issues/1747)

## What Changes

- Serialize requests that compete for the same idempotency scope before any controller mutation or mutation side effect runs.
- Make concurrent matching requests converge on one committed mutation and one stored response; waiting and later callers replay that response with `Idempotency-Replayed: true`.
- Preserve HTTP 409 `idempotency_key_reused` behavior when a key is reused outside its original request digest or authenticated account scope.
- Keep failed or abandoned attempts retryable: server failures and raised exceptions roll back the mutation, release serialization, and do not leave a permanent reservation.
- Define deterministic handling for successful, client-error, conflict, and server-error responses.
- Add a deterministic two-connection People-create regression proving that only one Person and one set of mutation side effects are committed.

### Non-goals

- Changing People fields, authorization, household isolation, or the public create response shape.
- Replacing endpoint-specific idempotency such as medication-take `client_uuid`.
- Generalizing all API writes into a new command framework.
- Changing the 24-hour idempotency retention period.
- Making separate accounts inside one household share a replayable response.

## Capabilities

### New Capabilities

- `api-mutation-idempotency`: Authenticated mutating API requests are serialized and replayed safely across concurrent deliveries.

### Modified Capabilities

None. This repository does not yet contain a generic API idempotency capability.

## Impact

- Changes the execution boundary in `Api::V1::BaseController` and `Api::IdempotencyStore`.
- Uses PostgreSQL transaction semantics to cover mutation, audit/change side effects, and response persistence as one idempotent attempt.
- Extends service and request coverage, including a real two-database-connection concurrency example against People creation.
- Preserves the existing `Idempotency-Key`, `Idempotency-Replayed`, response body/status, expiry, and PHI-safe conflict contracts.
