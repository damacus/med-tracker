## Context

`Api::V1::BaseController#with_api_idempotency` currently performs an unlocked `Api::IdempotencyStore#lookup`, yields to the controller action, and then calls `store!`. The unique index on `(household_id, key)` prevents two stored responses, but it runs after both requests may have created domain records, PaperTrail versions, access grants, and other side effects. `store!` then rescues the uniqueness failure, hiding the race without undoing the losing mutation.

People creation is the regression boundary because it writes a Person inside a controller transaction and also creates either delegated access or a household access grant. Person changes are audited through PaperTrail. A deterministic two-connection test can therefore prove that the generic middleware serializes both the primary record and its mutation side effects.

The idempotency table already stores household, account, credential attribution, method, path, digest, response status/body, and a 24-hour expiry. Its unique index defines the strongest current key namespace as `(household_id, key)`. The design must preserve that database invariant while preventing a response from being replayed to a different account.

`with_api_request_context` remains the outer API boundary. It authenticates and binds the tenant before idempotency runs, and it records an `api.request` security event for every HTTP delivery. Those per-delivery security events are intentionally distinct from mutation audit side effects: two HTTP attempts remain two access events even though only one Person mutation and one Person PaperTrail version may commit.

## Goals / Non-Goals

**Goals:**

- Serialize competing generic idempotent mutations before any domain write.
- Commit a successful mutation, its database side effects, and its stored response atomically.
- Let a waiter observe the committed response and replay it without executing the controller action.
- Preserve request-digest conflicts, tenant isolation, account isolation, the response policy, and PHI-safe errors.
- Release failed or abandoned attempts automatically so an iOS client can retry.
- Prove the behavior with real PostgreSQL connections rather than mocks or sequential calls.

**Non-Goals:**

- Changing endpoint-specific identities such as `MedicationTake.client_uuid`.
- Changing People authorization, fields, serialization, or delegation behavior.
- Changing the 24-hour expiry or introducing a general background reservation reconciler.
- Deduplicating per-delivery `api.request` security events.
- Adding response-header persistence; the existing generic store persists status and JSON body, while header parity remains a separate API-contract concern.

## Decisions

### 1. Use a PostgreSQL transaction-scoped advisory lock

`Api::IdempotencyStore` will expose one execution boundary that opens an Active Record transaction, acquires a PostgreSQL transaction-scoped advisory lock, performs lookup, yields the controller mutation when needed, and stores the response before commit. `BaseController` will use that boundary rather than coordinating separate `lookup` and `store!` calls.

The lock identity will be a versioned, one-way 64-bit digest of the routed `household_id` and raw idempotency key. It deliberately matches the table's existing `(household_id, key)` uniqueness namespace. Method, path, digest, and account are checked after the lock is held; they must not be part of the lock identity because competing reuse of the same database-unique key must serialize before conflict detection. The raw key and request content will not be logged.

The lock query and all application work must use the same checked-out database connection and transaction. PostgreSQL releases the lock automatically on commit, rollback, connection loss, or process failure. A hash collision can only serialize unrelated requests temporarily; the exact database lookup still decides replay or conflict.

Alternative considered: add pending/completed reservation states to `api_idempotency_keys`. Rejected because it requires a migration, nullable response fields, ownership/lease semantics, stale-reservation recovery, and a cleanup path even though PostgreSQL can provide lifecycle-safe serialization.

Alternative considered: rely on `INSERT ... ON CONFLICT` immediately before the action. Rejected because a placeholder row needs a durable pending state and a waiter protocol, and an insert inside the same transaction is invisible to waiters until commit.

Alternative considered: use a Ruby mutex. Rejected because it cannot coordinate multiple Rails processes or hosts.

### 2. Re-check replay or conflict only after serialization

Once the lock is held, the store will query `(household_id, key)` again:

- Matching account, method, path, and digest: render the stored status/body and set `Idempotency-Replayed: true`.
- Any account, method, path, or digest mismatch: render HTTP 409 `idempotency_key_reused` without running the action.
- No stored row: run the action exactly once inside the transaction.

Authenticated account is the durable credential scope. It survives access-token rotation while preventing another account in the same household from receiving a response that may contain health data. Existing session/app-token attribution remains stored for audit context. The database key namespace stays household-wide, so a different account reusing the same key receives a conflict rather than creating an independent row.

Alternative considered: scope replay to the exact `ApiSession` or `ApiAppToken` row. Rejected because a legitimate client retry should survive token rotation and because OAuth grants are not currently represented by a dedicated idempotency-table foreign key.

Alternative considered: migrate the unique index to account, method, and path. Rejected as an unnecessary contract expansion for #1747; retaining the existing household-wide key namespace is backward-compatible and safer for unknown clients.

### 3. Make response persistence part of the mutation transaction

For a new request, the action runs under the idempotency transaction. An eligible response is persisted before that transaction commits. `store!` must no longer swallow `RecordInvalid` or `RecordNotUnique`: after serialization, either indicates a violated invariant, and propagating it rolls back the mutation instead of leaving an un-replayable success.

Existing completed-response policy remains:

- HTTP responses below 500 are eligible for storage, except the middleware's own 409 key-reuse conflict, which never replaces the original row.
- Stored 4xx responses replay deterministically and must have no committed mutation side effect.
- HTTP 500-or-higher responses are not stored. Their database work is rolled back before returning the response, leaving the key retryable.
- Raised exceptions roll back automatically and are handled by the existing API exception path.

For a rendered server error, the wrapper will mark the transaction for rollback while preserving the rendered response. This prevents a controller that rendered a 5xx after partial work from committing that work.

Alternative considered: store the response after committing the action. Rejected because it recreates the crash window and duplicate-side-effect race that this change exists to remove.

Alternative considered: cache 5xx responses. Rejected because a transient failure would poison the key for the retention period and prevent a native client from recovering.

### 4. Test through the public People endpoint with two connections

The regression spec will use two independent Active Record connection-pool leases, thread barriers, and a bounded join/timeout. It will pause the first request after the post-lock miss but before the People mutation, start the second request, then release the first. The synchronization point will be a narrow test-only collaboration at the store boundary, not a production sleep.

Assertions will cover:

- one response is original and one has `Idempotency-Replayed: true`;
- both responses have the same status and JSON body;
- one Person exists;
- only one expected access grant or carer relationship exists;
- one Person PaperTrail create version exists; and
- no duplicate mutation side effect is present.

The spec will not expect one `api.request` security event, because access auditing correctly records both authenticated HTTP attempts. It will explicitly count the Person mutation audit version instead.

Alternative considered: unit-test two sequential `lookup` calls. Rejected because it cannot prove blocking, transaction visibility, connection ownership, or the absence of a real race.

Alternative considered: use sleeps to create the race. Rejected because timing-based concurrency tests are nondeterministic in local Docker and CI.

### 5. Keep authorization, tenancy, and privacy outside the lock mechanism

Authentication and household binding continue before the idempotency wrapper, so the lock is never acquired for an unauthenticated or unbound request. Tenant context and RLS stay active throughout the transaction. A different household derives a different lock identity and queries only its own idempotency row.

Conflict responses remain generic and do not echo the key, digest, request parameters, account, or stored response. Diagnostics may identify the error class and hashed lock version, but not raw identifiers or PHI.

## Risks / Trade-offs

- **[A slow mutation holds a database transaction and advisory lock]** → Scope locking only to keyed mutating requests, rely on existing database/request timeouts, and avoid external I/O inside keyed controller actions.
- **[A malformed controller path renders a 4xx after writing data]** → The generic boundary makes the entire request transactional; focused tests assert that stored 4xx outcomes leave no mutation side effects.
- **[An advisory-lock hash collision serializes unrelated keys]** → Use a versioned 64-bit cryptographic digest; exact lookup after locking prevents incorrect replay or conflict.
- **[Connection-pool pressure increases while duplicate requests wait]** → Native retries for the same key are expected to be rare and short; the waiting request trades one occupied connection for prevention of duplicate health-data writes.
- **[Changing `store!` from best-effort to atomic can expose latent persistence failures]** → Treat failure to persist a replayable success as a correctness failure and roll back; cover normal credential types and response bodies in focused service specs.
- **[Account-scoped replay differs from today's account-blind lookup]** → This closes a response-disclosure risk while retaining key reuse as a safe generic 409.

## Migration Plan

No database migration or backfill is required. Deploy the transaction/locking boundary and its regression coverage together. Existing completed idempotency rows remain readable and retain their expiry.

Rollback is code-only: restore the previous unlocked wrapper. No persisted data needs conversion, although rollback would reintroduce the concurrency race and should be reserved for an operational emergency.

## Open Questions

None. Response-header replay, including preservation of `ETag`, is deliberately not folded into #1747 and should be tracked as a separate public contract change if the iOS generated client requires it.
