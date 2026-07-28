## 1. Red: Prove the generic race and failure contracts

- [x] 1.1 Extend `spec/services/api/idempotency_store_spec.rb` with failing examples for account-scoped replay, cross-account conflict, transaction-scoped serialization, exact post-lock lookup, and non-sensitive lock identity.
- [x] 1.2 Add a deterministic request concurrency spec using two checked-out PostgreSQL connections and barriers to submit the same keyed People create concurrently.
- [x] 1.3 Assert the concurrency regression produces one Person, one applicable access grant or carer relationship, one Person PaperTrail create version, identical response status/body, and exactly one replay marker.
- [x] 1.4 Add failure examples proving response-storage errors and raised exceptions roll back all mutation side effects, rendered 5xx responses are not stored, and the same key can be retried afterward.
- [x] 1.5 Add request examples proving deterministic stored 4xx replay, same-key different-digest conflict, different-account conflict without response disclosure, and independent behavior across households.
- [x] 1.6 Run `task test TEST_FILE=spec/services/api/idempotency_store_spec.rb` and each new focused request spec; confirm the new examples fail for the intended unlocked behavior.

## 2. Green: Serialize lookup, mutation, and response storage

- [x] 2.1 Add a versioned, one-way advisory-lock identity derived only from household ID and idempotency key, without logging the raw key or request content.
- [x] 2.2 Refactor `Api::IdempotencyStore` to acquire the transaction-scoped PostgreSQL advisory lock and perform the post-lock lookup on the same checked-out connection and transaction.
- [x] 2.3 Require household, authenticated account, method, path, and request digest to match before replay; return the existing PHI-safe `idempotency_key_reused` conflict for every mismatch.
- [x] 2.4 Move new-request execution and eligible response storage into the serialized transaction so a stored success and every mutation side effect commit or roll back together.
- [x] 2.5 Stop swallowing idempotency persistence failures after serialization; propagate them so the transaction rolls back instead of committing an un-replayable mutation.
- [x] 2.6 Preserve stored sub-500 outcomes and middleware conflict behavior, while rolling back and leaving the key retryable for rendered 5xx responses and raised exceptions.
- [x] 2.7 Update `Api::V1::BaseController#with_api_idempotency` to use the store's single execution boundary and keep `Idempotency-Replayed: true` on replay.
- [x] 2.8 Run the focused store, People, sync-safety, tenancy, authorization, audit, and concurrency specs until green.

## 3. Refactor: Keep the boundary narrow and auditable

- [x] 3.1 Remove obsolete split lookup/store coordination while keeping authentication, tenant binding, controller authorization, and per-delivery `api.request` auditing outside the serialization mechanism.
- [x] 3.2 Confirm replay responses cannot cross authenticated accounts or households and all lock, conflict, exception, and diagnostic paths omit raw keys, parameters, response bodies, and PHI.
- [x] 3.3 Confirm no reservation-state migration, background cleanup worker, endpoint-specific idempotency change, or OpenAPI request/response shape change was introduced.
- [x] 3.4 Run `task rubocop` and correct any offenses without unrelated formatting.

## 4. Verification and handoff

- [x] 4.1 Run `task openspec:validate` and verify the implementation still matches the approved proposal, specification, and design.
- [x] 4.2 Run `task test` and confirm the full Docker-backed Rails suite passes.
- [x] 4.3 Run `task brakeman` and review any finding against the advisory-lock SQL, account/household replay boundary, and PHI-safe diagnostics.
- [x] 4.4 Review the final diff against issue #1747, confirm PR #1744 medication-take behavior is unchanged, and record response-header replay as separate follow-up work if the iOS client requires `ETag` parity.
