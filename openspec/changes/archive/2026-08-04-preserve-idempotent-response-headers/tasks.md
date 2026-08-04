## 1. Red: Specify Header and Legacy-Row Behavior

- [x] 1.1 Extend `spec/services/api/idempotency_store_spec.rb` with failing examples for ETag-only persistence, read-side allowlisting, account-conflict non-disclosure, unexpired legacy replay, and expired-row replacement.
- [x] 1.2 Extend `spec/requests/api/v1/people_controller_spec.rb` with failing create and update examples proving original and replayed ETags match without a second mutation.
- [x] 1.3 Extend `spec/requests/api/v1/idempotency_concurrency_spec.rb` with a failing assertion that the concurrent waiter receives the committed ETag.
- [x] 1.4 Run each focused spec through `task test TEST_FILE=...` and confirm the new examples fail for missing response-header persistence or expiry enforcement.

## 2. Green: Add Compatible Header Storage

- [x] 2.1 Add an additive Rails 8.1 migration for `api_idempotency_keys.response_headers` as JSONB with database default `{}` and `null: false`, then update `db/schema.rb` through the repository migration task.
- [x] 2.2 Add the explicit `ETag` response-header allowlist to `Api::IdempotencyStore`, persist only allowlisted present values, and apply the allowlist again when building a matching replay result.
- [x] 2.3 Restore replayable headers in `Api::V1::BaseController#with_api_idempotency` before rendering the stored response and adding `Idempotency-Replayed: true`.
- [x] 2.4 Remove an expired matching idempotency row only after acquiring the existing advisory lock and before post-lock lookup, keeping deletion and replacement in the same reservation transaction.

## 3. Refactor and Focused Verification

- [x] 3.1 Make response doubles and replay results expose headers consistently without weakening verifying-double coverage.
- [x] 3.2 Run the service, People request, and concurrency request specs together and confirm ETag parity, strict allowlisting, legacy behavior, account/household isolation, and exactly-once side effects.
- [x] 3.3 Run directly related sync-safety and idempotency request/service specs to confirm #1747 serialization and conflict behavior remain green.
- [x] 3.4 Run `task rubocop` and fix any offenses without broadening scope.

## 4. Full Validation and Handoff

- [x] 4.1 Run `task openspec:validate` and confirm the change artifacts are valid.
- [x] 4.2 Run the full `task test` suite and record any expected pending examples separately from failures.
- [x] 4.3 Review the final diff against issue #1749 and the OpenSpec requirements, including the 24-hour legacy transition and exclusion of all non-allowlisted headers.
- [x] 4.4 Commit with a Conventional Commit message, push the branch, open a focused pull request linked to #1749, and monitor every required check to green.
