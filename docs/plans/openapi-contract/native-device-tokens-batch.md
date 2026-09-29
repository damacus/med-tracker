# Native device token batch

Follow-on within the user-authorised five-percentage-point budget. Usage was
95% when this pair was selected, against 94% at the start. Complete after the
notification preference slice; no further families in this batch.

Scope: `createNativeDeviceToken` and `deleteNativeDeviceToken`, both in the
original 89-operation baseline. Registration stores account-owned tokens;
revocation is idempotent and cannot remove another account's token. Neither
operation sends notifications or calls a push service.

Acceptance: exact documented request validation; empty 201 registration and
204 revocation; repeat registration updates the same account's token; global
token uniqueness rejects another account's claim with 422, including races;
platform, APNs environment and user-agent handling follow existing source
semantics. Verify authentication, household membership and policy checks,
rate-limit wiring, and token secrecy in errors and audit records. Preserve
omitted optional values as the source controller does. No schema changes.

Sol owns production, necessary specification clarifications and runner wiring.
Luna owns `openapi_native_device_tokens.rs`, proving missing-route RED before
implementation. The orchestrator owns independent review and publication.
Reuse existing helpers and isolated Compose infrastructure. Keep checks
focused, then run applicable final gates once for both slices.

## Completion evidence

Initial RED: registration returned 404 instead of 201 in isolated project
`mtcontract-c128dd44e7cf48ca`, log `1790413099_task_api_08f7e1.log`.
Final isolated GREEN: all five HTTP groups passed in
`mtcontract-0e56dfe2180546b4`, log `1790413923_task_api_08f7e1.log` under
`/Users/damacus/Library/Application Support/rtk/tee/`. Cleanup completed.

Independent review verified account ownership, serialized same-account
updates, database-enforced uniqueness with savepoint recovery, empty success
responses, omitted APNs preservation and generic token-free errors/audits.
Whitespace-only tokens are rejected without trimming legitimate values.
Tests prove concurrent claims have exactly one winner and foreign-account
revocation leaves the owner's row intact. Production SQL logging is disabled.

Review fixed a lazy-thread test deadlock, reordered replay assertions, and
removed an unsupported test requirement for a response header on pre-auth
errors. Required error body request IDs remain checked. Selected compilation,
formatting, Clippy, 15 API unit tests and runner dispatch/cleanup checks passed.
No Rails application code changed, so the Rails suite was not rerun locally.
