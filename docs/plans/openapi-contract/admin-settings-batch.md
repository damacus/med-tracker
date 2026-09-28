# Household administration settings

Implement and verify getHouseholdAdminSettings, updateHouseholdAdminSettings
and replaceHouseholdAdminSettings before proceeding to medication workflows.

Response is exactly id, name, slug, timezone, subscription_plan and updated_at
inside data. Owners and administrators can read/write; ordinary members,
foreign households and invalid credentials are denied. PATCH/PUT merge name,
timezone and subscription_plan; preserve slug and omitted fields. Enforce the
documented strict schema, nonblank values and free/family_plus plans. Return
documented malformed, validation, conflict and rate-limit envelopes.

Preserve successful-update security audits and transactional behaviour.
Implement shared idempotency if missing: same account/key/request replays
the response without duplicate writes/domain audits; different request or account
conflicts; expiry and current authorization apply. New credentials for the
same account can replay after fresh authentication and authorization. Inspect Rails
Api::IdempotencyStore and existing Rust port code for compatible prior art.
Do not reproduce an authorization defect on replay.

Sol owns production, necessary contract clarification and Task/runner wiring.
Luna owns openapi_admin_settings.rs. Compile and run initial RED before
production changes; expand tests in parallel. The orchestrator reviews both
lanes before final isolated acceptance and owns evidence/publication.

Use the established API Task commands, selected contract compilation and
isolated Compose projects. Final gates include API formatting, Clippy/unit
tests, selected acceptance, runner checks if changed and documentation build.
Record exact evidence and remaining gaps; no route-only completion claims.

## Completed evidence

Initial compiled RED ran in `mtcontract-e13f4e0ce366407d`: the owner GET
returned 404 instead of 200; runner exit 201. Log:
`/Users/damacus/Library/Application Support/rtk/tee/1790584311_task_api_4363fe.log`.

Final isolated acceptance passed all ten groups in
`mtcontract-01f29823c5dd4613`, exit 0, with complete project cleanup. Log:
`/Users/damacus/Library/Application Support/rtk/tee/1790584913_task_api_a358cd.log`.
Independent requirements and code review passed after adding explicit
PATCH/PUT authentication, household, malformed JSON and PUT replay checks.
The tests verify concurrent identical requests produce one domain audit,
same-account replay through a second credential, denial after demotion,
expired-key reuse and invalid-write nonmutation.

API formatting, Clippy, 15 unit tests, selected contract compilation,
runner syntax and success/failure cleanup checks, documentation build and
diff checks passed. No Rails application code, database schema or dependencies
changed. The idempotency helper is currently wired to settings; other write
families reuse it as they are completed. Stored Rails digest interoperability
is not claimed.

This batch brings the fixed scope to 22/118 verified, with 96 remaining;
the original baseline is 22/89 verified, with 67 remaining. The 118 operation
IDs, methods and paths are unchanged. The next batch is assignment writes.
