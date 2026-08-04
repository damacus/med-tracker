## Context

`Api::V1::BaseController#with_api_idempotency` delegates keyed mutations to `Api::IdempotencyStore#with_reservation`. The store opens a `requires_new` transaction, acquires a PostgreSQL transaction-scoped advisory lock for the `(household_id, key)` namespace, performs the post-lock lookup, runs the controller mutation on a miss, and persists response status/body before commit. A replay renders that stored status/body and adds `Idempotency-Replayed: true`.

Resource responses are rendered through `BaseController#render_resource`, which sets an ETag derived from the record class, database ID, and `updated_at`. The OpenAPI contract requires ETag on successful create/update resource responses. `Retry-After` is the only other named OpenAPI response header, but Rack Attack produces it before the Rails controller and idempotency transaction; it is not part of a stored mutation response. Cookies, credentials, request identifiers, rate-limit metadata, cache controls, and arbitrary application headers must not enter the durable idempotency row.

The table already records `expires_at` using a 24-hour retention value, but the reservation path does not currently remove an expired row before lookup. Adding a header column with an empty default keeps existing rows readable, but those rows cannot be backfilled because the original ETag was never captured.

## Goals / Non-Goals

**Goals:**

- Preserve the original ETag across normal and concurrent idempotent response replay.
- Store and restore only explicitly allowlisted, non-sensitive response headers.
- Keep response storage atomic with the mutation under the #1747 reservation transaction.
- Give existing headerless rows a data-safe, bounded transition without re-running a completed mutation.
- Preserve account matching, household isolation, conflict behavior, rollback, audit behavior, and exactly-once mutation side effects.

**Non-Goals:**

- General response caching or arbitrary/header-prefix persistence.
- ETag reconstruction from stored JSON or endpoint-specific record lookup.
- A background cleanup process or a different retention duration.
- Persistence of `Retry-After`, `Set-Cookie`, `Authorization`, `X-Request-Id`, cache headers, or rate-limit headers.
- Changes to medication-take `client_uuid` idempotency or any endpoint response body/schema.

## Decisions

### 1. Add a compatible JSONB response-header column

Add `response_headers` to `api_idempotency_keys` as `jsonb`, `null: false`, with database default `{}`. The migration is an additive expansion that permits old and new application versions to coexist: old code ignores the column, while new code accepts empty hashes from legacy rows.

Alternative considered: one nullable ETag string column. Rejected because the issue requires an explicit replayable-header contract that can grow deliberately without another schema migration, while JSONB still stores only allowlisted scalar values.

Alternative considered: backfill ETags. Rejected because the generic row does not retain the record class/database ID pair required by `Api::RecordEtag`, and inferring it from paths or response JSON would be endpoint-specific and unsafe.

### 2. Centralize a case-specific allowlist in the idempotency store

`Api::IdempotencyStore` will define `REPLAYABLE_RESPONSE_HEADERS = %w[ETag].freeze`. When persisting a completed response, it will select only those exact names from `response.headers`; missing values are omitted. When returning a matching replay result, it will reapply the same allowlist to stored JSON before handing headers to the controller. This read-side filtering prevents manually altered or historical database content from bypassing the current allowlist.

Alternative considered: store all headers except a denylist. Rejected because framework and middleware headers can contain cookies, request correlation data, cache policy, or future sensitive metadata that a denylist would fail to anticipate.

Alternative considered: include `Retry-After`. Rejected because rate limiting occurs in Rack Attack before controller idempotency and a replayed completed mutation must not carry stale retry timing.

### 3. Restore stored headers before rendering the replay body

The replay result will expose an immutable header hash alongside the stored record and replay/conflict flags. `BaseController#with_api_idempotency` will set each returned header before rendering the stored JSON/status, then set `Idempotency-Replayed: true`. Conflict and miss results expose an empty header hash.

This keeps durable-header selection inside the store while leaving HTTP response application at the controller boundary. The action is still skipped for a replay, and no new domain write, PaperTrail version, access grant, or audit mutation side effect occurs.

### 4. Enforce existing expiry only after acquiring the reservation lock

After acquiring the transaction-scoped advisory lock and before lookup, the store will delete a matching row only when `expires_at <= Time.current`. It will then perform the normal post-lock lookup and mutation/store flow. Deletion and replacement remain inside the same transaction and lock namespace, so two callers cannot both reopen the expired key.

Fresh legacy rows with `{}` headers remain replayable without an invented ETag until their recorded expiry. This prioritizes exactly-once mutation safety over immediate header parity for responses created before deployment. Because expiry is enforced at the next keyed attempt, legacy rows age out within the already documented retention window without a background worker.

Alternative considered: delete all existing rows in the migration. Rejected because a retry could execute an already committed mutation again.

Alternative considered: return a new error for headerless rows. Rejected because that adds a public response contract and makes previously replayable requests fail even though the stored status/body remain valid.

### 5. Verify public create, update, and concurrent replay

Request coverage will prove a People create and update store the original ETag and return it unchanged on replay. Service coverage will prove non-allowlisted headers are omitted on write and read, legacy rows remain safe, expired rows can be replaced, and mismatched accounts receive no stored headers. The existing two-connection concurrency request spec will include ETag parity while retaining its exactly-one-mutation assertions.

The People endpoint remains the public regression boundary because it exercises resource ETags, transactional delegation/access-grant side effects, PaperTrail, household routing, and the generic idempotency wrapper together.

## Risks / Trade-offs

- **[Fresh legacy rows can replay without ETag during the rollout window]** → Preserve the mutation result rather than risking duplicate execution; enforce the recorded 24-hour expiry so the window is bounded.
- **[A future developer adds a sensitive header]** → Require an explicit constant change and retain write-side plus read-side allowlist tests.
- **[Expired-key deletion weakens serialization]** → Perform it only after the existing advisory lock and inside the same `requires_new` transaction as replacement.
- **[JSONB values are not guaranteed to be strings]** → Persist only values read from the response header interface and ignore non-allowlisted stored keys on replay.
- **[Migration rollback while new code is deployed removes a required column]** → Roll application code back before reversing the schema migration; leaving the additive column in place is safe.

## Migration Plan

1. Deploy the additive `response_headers` JSONB column with default `{}` and `NOT NULL`.
2. Deploy application code that writes/reads the allowlist and expires matching old rows under the reservation lock.
3. Existing fresh rows continue their current status/body replay behavior; newly created rows include ETag where the response provides one.
4. After the existing 24-hour retention window, every reusable row has passed through the new writer.
5. Roll back application code first if needed. The unused additive column can remain; reverse the migration only after no deployed code references it.

## Open Questions

None. `ETag` is the only response header currently required by both a successful idempotent resource mutation and the generated-client concurrency contract.
