# OpenAPI-first API checkpoint

Specification: `docs/api/openapi.v1.yaml`, SHA-256 `89a0710edb76ba096c0e5e79c931aef9004577ac2b41d4eaecdd64cdbfbe2016`. The operation map is `operations.jsonl`; `operation-filter.jq`, `route-sources.json`, and `test-evidence.json` are its inputs. Rebuild it with:

```fish
yq -o=json '.' docs/api/openapi.v1.yaml | jq -c --slurpfile routes docs/plans/openapi-contract/coverage/route-sources.json --slurpfile evidence docs/plans/openapi-contract/coverage/test-evidence.json -f docs/plans/openapi-contract/coverage/operation-filter.jq > docs/plans/openapi-contract/coverage/operations.jsonl
```

The specification has 118 operations across 77 paths, with 227 component schemas,
12 component responses and 772 response cases. Of those operations, 47 are fully
verified and 71 remain incomplete: 53 absent routes, 10 present but unverified,
and eight partially verified location operations. The original 89-operation
baseline is now 36 complete and 53 remaining. Both baseline files are retained;
the operation IDs, methods and paths still match the fixed 118-operation scope.
Route presence and older Rails-derived tests alone receive no completion credit.

## Medication validation and pause history

This batch verifies seven existing medication/take operations and adds eight
pause-history, pause/resume and ordering operations. The combined medication
Task returned exit 0 in `mtcontract-765d769dce6a4deb`: all four selected binaries
completed (nine read, nine stock, six dose-write and nine OpenAPI tests).
Pause acceptance returned exit 0 in `mtcontract-9e299c71c4af4a4f` with all ten
defined tests selected. Both projects were cleaned up. The complete green test
summaries were not retained in RTK tee files; successful task completion was
observed. Medication build trace: `1790590703_task_api_1afb52.log`.

Medication strict-request RED is retained in `1790589638_task_api_9b6f8c.log`.
It exposed unknown take fields creating records, invalid scalar inputs changing
resources and malformed JSON bypassing structured errors. Validation now
rejects unknown fields, forbidden nulls, invalid identifiers and decimal forms;
exactly representable trailing zeros remain valid. Invalid pagination returns
422. Three older expectations were corrected to the fixed schema: required
reorder threshold, invalid pagination and unknown source-type enum rejection.

Pause missing-route RED is retained in `1790588499_task_api_89fa45.log`;
bodyless-request RED is in `1790589365_task_api_1a8aec.log`. Tests cover current
person grants even for owners, administrator success, history and actor fields,
repeated and concurrent pauses, stale ETags, old-period resume safety, bodyless
requests, ordering persistence and all eight rate-limit paths. Period history
uses database pagination and batched actor loading. Period versions and source
sync events remain distinct. Client timestamps and invalid reorder directions
are explicitly rejected rather than silently ignored.

Independent code and test review passed. API check, Clippy, formatting, 15 unit
tests, selected compilation, runner dispatch/failure cleanup and documentation
checks passed. No Rails application code, database schema or UI files changed.
Occurrence work is prepared separately and receives no credit in this batch.

## Assignment and schedule management

The combined batch adds six write operations and verifies four existing reads.
Assignment acceptance returned exit 0 in `mtcontract-23218744c10841cd` with all
11 defined tests selected. Its exact test summary was truncated in the tool
output and not retained; the successful task result and cleanup were observed.
Schedule acceptance passed 10/10 in `mtcontract-2f0615fdd5a547ab`. Both isolated
projects were removed. Build traces are `1790587777_task_api_760696.log` and
`1790587973_task_api_ad2553.log` in the RTK tee directory; these are build logs,
not retained test summaries. True no-op RED is retained in
`1790587311_task_api_749080.log` and `1790587567_task_api_3794f0.log`.

Independent review covered access grants, strict requests and responses,
source dosage visibility and binding, transactional versions and sync events,
idempotency, stale ETags and one-winner concurrent updates. Tests also cover
unchanged updates, pagination, date-dependent schedule activity and direct
rate-limit wiring. API formatting, Clippy, 15 unit tests, selected test
compilation/formatting and runner dispatch/failure-cleanup checks passed.

Documented corrections reject silent dose rounding, fractional integer-hour
truncation, invalid recurrence values and reassignment of medical history to a
different person. Unchanged writes preserve timestamps and avoid spurious
version/sync events. Existing source dosage bindings remain intact. The shared
idempotency helper now also covers these writes and authorised validation
responses. The paused web consumer's use of undocumented stock/permission
fields remains a separate UI integration dependency; UI parity is not claimed.

## Household administration settings

GET, PATCH and PUT passed ten isolated HTTP acceptance groups in project
`mtcontract-01f29823c5dd4613`, which was cleaned up. Independent requirements
and code review covered strict response/request fields, owner/administrator
authority, partial updates, slug preservation, invalid-write nonmutation and
successful-update audit records. Direct checks cover authentication, foreign
and missing households, malformed JSON, validation, conflict and rate limits.

The shared idempotency helper serialises household mutations, rechecks current
authentication and role before replay, and scopes keys to household/account,
method, path and request content for 24 hours. Tests prove replay through a
second valid credential for the same account, denial after demotion, conflict
on changed account or payload, one domain audit for concurrent identical
requests, and reuse after expiry. PATCH and PUT have direct replay assertions.
The helper is used by these settings operations; this does not claim that all
other mutations use it or that stored Rails request digests are interchangeable.

## Web push subscriptions

The next two-operation tranche completed web push registration and revocation.
All five HTTP groups passed in `mtcontract-48617928f63f4ae9`, log
`1790424187_task_api_694219.log` in the tee directory below. Independent review
and focused Rust/runner checks passed; see `push-subscriptions-batch.md`.
Proof covers provider allowlist boundaries, account-owned replay, unique
concurrent claims, encoded-query deletion, revocation of existing unsupported
provider records, secret redaction and rate limits. No push delivery was added.

## Notification preferences and native device tokens

The budgeted follow-on completed five missing operations using separate Sol production and Luna test lanes. Notification preferences passed both HTTP groups in `mtcontract-26b8a402e0c5404a`, log `1790412885_task_api_624fdf.log`; native tokens passed all five groups in `mtcontract-0e56dfe2180546b4`, log `1790413923_task_api_08f7e1.log`. Logs are under `/Users/damacus/Library/Application Support/rtk/tee/`. Both isolated projects were cleaned up. Initial missing-route RED, review corrections, exact scope and intermediate test mistakes are recorded in `notification-preferences-batch.md` and `native-device-tokens-batch.md`.

Preferences verify defaults, merged updates, nullable and strict time values, view/manage permissions, no-op stability and private person-scoped sync metadata. Native tokens verify account-owned replay updates, global uniqueness, one-winner concurrent claims, foreign-account nonmutation, idempotent deletion, empty success bodies and audit redaction. All five operations have direct nonloopback 429 assertions; common auth, audit and limiter helpers reuse existing proofs. Independent requirements/code review passed. Formatting, Clippy, 15 API unit tests, selected contract compilation and runner dispatch/cleanup checks passed. No dependencies, Rails application code or database schema changed in this batch.

## Profile and person evidence

The four operations in `people-batch.md` passed 17 isolated HTTP tests in project `mtcontract-eae843f4fe5d408a`, log `/Users/damacus/Library/Application Support/rtk/tee/1790410591_task_api_2302ff.log`; the project was cleaned up. The initial four-route run failed with HTTP 405. Subsequent runs exposed a portable-ID detail-read defect and unnecessary events on unchanged updates, both fixed, plus test fixture/cleanup mistakes that were corrected without weakening assertions.

The final suite covers strict profile/person schemas, real cross-household own-profile identity and locations, creation permissions, email normalization and concurrent uniqueness, age/capacity/carer rules, Home assignment, linked and restored self grants, version increments, correlated domain audit/history/sync, merged PATCH/PUT updates, unchanged-update stability, invalid-write rollback, household/auth boundaries and rate-limit wiring. Production and test files had separate owners and independent orchestrator review. Review corrections included adult capacity validation, dependent grant linkage, source-compatible email validation and locked permission-version increments.

Proof is reused where the implementation is shared: PATCH numeric IDs and PUT portable IDs exercise the same update helper; role-independent manage-grant checks are shared; administrator validation/race requests and successful owner writes exercise the shared owner/administrator create branch. This does not claim a separate administrator-201 case. Each route has a direct 429 assertion; error-body/header semantics reuse the existing limiter tests. These reviewed combinations close the bounded documented contracts without testing every redundant permutation.

This run closed nine original-baseline operations in approximately 48 minutes to verified HTTP results: five dosage closures and this four-operation family. Luna completed dosage and deterministic evidence work; person fixture complexity warranted moving its test lane to Sol. The two Sol lanes then completed production and tests separately. OpenSpec housekeeping and additional API families remained deferred.

## Account session evidence

The three operations at [OpenAPI line 150](../../../api/openapi.v1.yaml) use the exact `AuthSession` and collection schemas and inherited bearer security for list and selected revoke. The Rust routes and SeaORM handlers are in `rust/api/src/auth_sessions.rs`. Source credential namespaces and list/logout semantics were documented in OpenAPI before contract tests. API sessions and app tokens manage their own account's API sessions; mobile OAuth manages its own account's mobile grants; integration OAuth is accepted for logout only. Nonrevoked API sessions remain listed after access or refresh expiry, while expired bearer credentials cannot authenticate a new list or selected revoke request.

Eight contract tests in `rust/contract-tests/tests/openapi_auth_sessions.rs` verify exact response fields and timestamps, own-account list scope, foreign-account 404 and nonmutation, durable selected/current revocation, idempotent empty 204 responses, invalid and inactive credential denials, namespace isolation with colliding IDs, target-household audit records, malformed IDs, expired-login targets, configured 30-day mobile maximum age, concurrent revocation audit uniqueness, and nonloopback 429 for all three methods without revocation. A pure API unit test verifies calendar-month app-token cap at a leap-day boundary. The test helper uses disposable credentials and checks that active refresh lookup no longer finds a revoked session. The initial compiled RED failed six tests on absent routes before deeper assertions, log `/Users/damacus/Library/Application Support/rtk/tee/1790373634_task_api_442df8.log`. The first GREEN passed 7/7, log `/Users/damacus/Library/Application Support/rtk/tee/1790374341_task_api_442df8.log`. Final isolated GREEN passed 8/8 in project `mtcontract-580c08dfe7ae4d14`, log `/Users/damacus/Library/Application Support/rtk/tee/1790374763_task_api_442df8.log`; its project was removed. The API unit suite passed 15/15, and the selected-test compile Task and runner dispatch/failure-cleanup shim test passed. All three declared session operations are marked fully verified in `test-evidence.json`. Randomized fuzzing and every timing interleaving remain outside the bounded contract proof.

## Dosage option evidence

The five operations at [OpenAPI line 1688](../../../api/openapi.v1.yaml) use `DosageOption`, its collection/resource envelopes, create/update attributes, `PaginationMeta`, and inherited bearer security. Their role, visibility, validation and parent-sync rules are now documented from the existing Rails controller and model rules. Thirteen contract groups in `rust/contract-tests/tests/openapi_dosage_options.rs` prove owner and administrator writes, active member reads scoped to visible parent medications, hidden detail 404, denied member writes, exact valid resource shapes, decimal strings, both ID forms, filters and pagination, malformed and invalid requests, missing/invalid bearer, foreign-resource 404, ETags, optional If-Match, stale supplied If-Match 409, and a concurrent same-ETag write race. All five methods return the shared strict 429 response under nonloopback exhaustion.

The earlier route and owner-path RED/GREEN evidence remains in the initial dosage review. This completion tranche first failed four real policy/parent/validation cases in isolated project `mtcontract-734187e1b2d54dc1` (log `1790375418_task_api_d3a3cd.log`). A preliminary GREEN passed 10/10 in project `mtcontract-92344c9e319541b7` (log `1790375869_task_api_d3a3cd.log`). Review then added two regressions: missing correlated parent Medication version and partial update of a legacy row with missing unit returning 500 rather than 422. They failed in project `mtcontract-971e9f9434634163` (log `1790376059_task_api_d3a3cd.log`). Final isolated acceptance passed 11/11 in project `mtcontract-1d35ec94c1de4bf9` (log `/Users/damacus/Library/Application Support/rtk/tee/1790376248_task_api_d3a3cd.log`); the project was removed.

SeaORM updates lock household, parent medication and dosage in that order. A single aggregate query sums tracked option stock and reorder thresholds. Creation clears the parent's single-dose amount. Tracked inventory changes update parent stock, reorder threshold and last-restock high-water mark; resetting the final tracked option restores null stock and baseline with zero threshold. Parent and option changes, correlated sync events and PaperTrail-compatible versions commit atomically. Adult and child default conflicts, aggregate overflow and invalid values return 422 without data or event changes. Exact decimal parsing rejects unrepresentable storage precision instead of allowing PostgreSQL rounding. The database now rejects NULL for all six required dosage fields; Rust SeaORM fields use non-optional types. The earlier legacy-row repair case is retained only as historical RED evidence and is replaced by direct database rejection coverage.

The shared Rust household authenticator now accepts account-bound app tokens after checking revocation, stored expiry, the calendar-month age cap, active account and user, lockout, membership status and permissions version, and an operational issuing household. Successful use refreshes `last_used_at` no more than once per five minutes. API request audits identify the credential as `api_app_token:<id>`. SMART integration grants remain outside ordinary household API authority. The dosage HTTP matrix verifies owner, administrator and member access, invalid token states, cross-household 403, missing household and foreign medication 404, and distinct audit identity. Initial RED included real app-token 401 failures in project `mtcontract-12b621a5edb84e30` (log `/Users/damacus/Library/Application Support/rtk/tee/1790387622_task_api_d3a3cd.log`). Final isolated dosage GREEN passed 13/13 in project `mtcontract-2407927431414afe` (log `/Users/damacus/Library/Application Support/rtk/tee/1790388342_task_api_d3a3cd.log`); session regression passed 8/8 in project `mtcontract-df1315fee61146cb` (log `/Users/damacus/Library/Application Support/rtk/tee/1790388083_task_api_442df8.log`). Both projects were removed.

The user chose database enforcement over a special read-time error. Migration `20260926000000_enforce_required_dosage_fields.rb` and `db/schema.rb` now enforce NOT NULL for amount, unit, frequency, default_max_daily_doses, default_min_hours_between_doses and default_dose_cycle. The migration has no backfill or default and fails with a column-specific message when existing NULL values need repair. The Rails migration spec checks all six fields, PostgreSQL 23502 rejection, valid-row preservation and rollback of earlier constraints on failure. Rust dosage acceptance checks the same six 23502 failures through the isolated database. Apply the migration before deploying the Rust binary with non-optional entity fields; no production migration was run here.

The final shared-auth gap closed on 26 September: the same valid app token gets 200 while its household is active, 401 while archived, then 200 after restoration. The guard restores exact household state even on assertion failure; the isolated runner uses one test thread. Combined with the existing five-method auth wiring assertions, the full 14/14 dosage run verifies all five operations. Project `mtcontract-63a3acc811174d13` completed and cleaned up; log `/Users/damacus/Library/Application Support/rtk/tee/1790408394_task_tes_d27969.log`. Independent test review passed. Closing this batch took approximately 13 minutes from assignment to verified result; two earlier runs exposed fixture-assumption mistakes, not production defects.

## Location policy, retention and rate evidence

The eight location operations start at [OpenAPI line 1012](../../../api/openapi.v1.yaml). Their request schemas are `LocationCreateRequest`, `LocationUpdateRequest` and `LocationMembershipRequest` (from line 5969); response schemas include `Location`, `LocationMembership`, their envelopes and `PaginationMeta` (line 4383). The routes are in `rust/api/src/lib.rs`; SeaORM reads and pagination validation are in `rust/api/src/read_resources.rs`; authorization, ETag checks and transactional cascade are in `rust/api/src/locations.rs`. Rate limits are in `rust/api/src/rate_limit.rs`. The three spec-derived test files are `openapi_locations.rs`, `openapi_location_writes.rs` and `openapi_rate_limit.rs` under `rust/contract-tests/tests`. `test-evidence.json` records exact statuses, behaviours and omissions for each operation.

Reads verify strict response envelopes, field formats, active-member access, numeric and portable IDs, wrong-household 403, foreign-resource 404, pagination and timestamp filters. Writes verify owner and administrator permissions, current person manage grants, malformed bodies and IDs, strict request properties, nullable versus absent description, empty 204 bodies, If-Match 428/409 and an atomic same-ETag update race. Seven independent retained-history sources block deletion with 422. An unretained graph cascades the location, membership, medication, dosage, schedule and person assignment. A late foreign-key conflict rolls all deletes back. A concurrent take/delete test requires either retained history with 422 or a failed take insert with PostgreSQL 23503. A nonloopback HTTP test verifies 429 and the strict rate body and headers on the location list. Controlled-clock unit tests verify all five API operation limits, expiry, concurrent counters, capacity and trusted-proxy handling. These assertions do not establish every possible location status or schema combination.

Initial write-route RED: seven compiled tests failed on absent POST/PATCH/PUT/DELETE methods, log `/Users/damacus/Library/Application Support/rtk/tee/1790365256_task_api_7d11fd.log`. Earlier read/write GREEN and malformed pagination RED/GREEN are in logs `1790365826`, `1790366008`, `1790366176` and `1790366360` under the same rtk tee directory. Policy RED: administrator create returned 403 instead of 201; no person grant returned 403 instead of 404, log `1790368751_task_api_7d11fd.log`. Retention/cascade/race RED reached the previous foreign-key-only deletion path and returned 409, log `1790369300_task_api_7d11fd.log`. The source-dosage graph then exposed deletion ordering 422 instead of 204; the separate HTTP rate test saw no 429 in 601 requests, log `1790369621_task_api_7d11fd.log`. Final isolated GREEN passed 18 write, eight read and one HTTP rate tests, including malformed DELETE ID, PUT stale version and membership household paths, log `/Users/damacus/Library/Application Support/rtk/tee/1790370643_task_api_7d11fd.log`. Its owned Compose project `mtcontract-2e1eae7a3e7e43cc` was removed.

The limiter uses an atomic mutex over at most 65,536 live IP/rule buckets per process. An expiry heap reclaims old windows without scanning the map on each request; capacity saturation rejects new clients with 429. The socket peer is authoritative by default. `API_TRUSTED_PROXY_IPS` explicitly names trusted proxy IPs; forwarding headers from other peers are ignored. Direct loopback is exempt, while configured loopback proxies are counted even when forwarding headers are missing or malformed. The default deployment has no trusted proxy list. The limiter contains no test-only HTTP clock endpoint or raised production threshold.

## Remaining gaps

- **Implementation:** 53 documented method/path pairs still have no Rust route. Their identities remain in `operations.jsonl`.
- **Untested behaviour:** 10 other route matches have no spec-traced runtime assertion. Eight location methods remain partial; `test-evidence.json` lists their remaining cases. All five dosage operations now have full bounded contract proof.
- **Outside this API tranche:** Approved permission, retention and rate rules were documented from Rails policy, controller and initializer sources before Rust tests. Browser support sessions remain outside bearer authority. Fixture SQL only provisions isolated records. The Rails dosage schema changed in this remediation; no Rails controller, model or UI behavior changed. Web-only Rack Attack rules and other API operations remain separate work.

The isolated runner still rebuilds broad API and contract-test sources. Its cost is a tooling follow-up; it did not narrow the spec-derived assertions.
