# Later journey acceptance packet

The same writer continues only after the preceding journey is independently
accepted. This packet preserves scout findings without another research round.

## E: stock

Use the existing shared API. `adjust_inventory` accepts absolute nonnegative
decimal `new_quantity` and reason, reauthorises and locks household/medication.
`stock_removals` accepts positive quantity, reason, lowercase UUID submission_id,
optional note and dosage_id. Exact supported reasons: dropped, damaged, expired,
discarded, lost, transferred_out, other. Notes are at most 1000 characters.
For dosage options select a tracked option, reject insufficient stock, preserve
null versus zero, and refresh the aggregate parent atomically. Prove payload
conflict/replay and reauthorisation. Parent absolute adjustment must not leave
tracked-option aggregate inconsistent: expose it only for scalar mode. Option
stock adjustment selects/links its real dosage edit API with original ETag;
see stock-ruling.md. The parent endpoint has no dosage_id support. Existing
scalar adjustment drops its submitted audit reason; fix #2350 via RED/GREEN
before claiming that the reason was recorded. Option edits have no reason field.

Order marks status/timestamp and optional supplier, quantity and expected date.
Receipt takes an empty object and marks received; it does not add quantity.
Present receipt status and stock adjustment as distinct user actions. Verify
immediate read-back, audit/sync and dose-consumption regression. Do not claim
idempotency or stale-write protection on endpoints which do not support them.
Cover forged related IDs, permission loss, CSRF, rejected drafts, all locales,
keyboard and mobile. Relevant existing targets include medication_stock and
openapi_stock_workflows; inspect actual runner target rather than assuming a
task with an API name tests Rust.

## F: assignments and schedules

Before adding to mixed large modules, split person_medication_writes,
schedule_writes and pause_lifecycle by responsibility, preserving comments and
observable behaviour. No arbitrary line-count target. Keep transactions, locks,
original ETag, reauthorisation, audit and sync/replay contracts intact.

Public person-medication endpoints support collection GET/POST, member
GET/PATCH/PUT, pause/resume and reorder. No public DELETE. Management requires a
current person manage grant, not merely a household role. Person_id and
medication_id are strings; optional dosage option must belong to the medication
and match the submitted snapshot. Dose has two decimal places; min hours is a
positive whole decimal string. Default-option resolution is already supported.

Implement direct assignment and all seven schedule types: daily, multiple_daily,
weekly, specific_dates, prn, tapering and every_other_day. Monthly is a dose
cycle, not an eighth schedule type. Typed draft controls must survive round-trip
and failed validation, including every taper step's bounds, dose/unit/limits
and times. Retain effective dates, timezone/DST, pause/resume intervals and
history. PRN must not invent routine expected doses. Review actual dashboard
eligibility at taper/date/pause boundaries, not just saved JSON.

Prove permission loss, view-only denial, foreign person/medication/option,
stale/missing browser tokens and idempotency payload conflict. All seven editors
need actual desktop/mobile browser persistence/edit evidence with translated
errors and labels. Existing targets include openapi_person_medication_writes,
openapi_schedule_writes and schedules; inspect their actual runner targets.

## G: existing baseline acceptance gaps

Complete the final large-module candidate in #2348, portable_imports.rs, after
the household journeys. Keep this a behaviour-preserving extraction: separate
dry-run/apply orchestration, field contracts, row validation, identity resolution
and persistence using actual cohesive boundaries. Establish its existing
observable baseline, preserve comments and public exports, test its import
regressions and obtain independent review. This fulfils the earlier personal
review request before any production/main landing; it does not broaden import
or authentication capabilities or authorise cutover.

Issue #2347 records identical pre-refactor and candidate failures. Do not weaken
assertions merely to turn green. Repair direct SQL dosage fixture setup with
required default_dose_cycle. Verify microsecond timestamp semantics without
reducing ETag precision. Preserve optional barcode/friendly_name/warnings in
OpenAPI medication responses. Check Cache-Control semantically for private and
no-store rather than rejecting stronger equivalent medical-data protection.

Make documented Rails/OpenAPI rulings for location ordering, paused-source stock
eligibility and out-of-range pagination. Re-run original limited-view dashboard
tests after the accepted dashboard fix. Read source_capabilities, web_reads_api,
medications, read completion and affected broad suites; preserve tests for the
real contract. Promote new accepted browser/HTTP tests into actual CI selectors
so publication is reproducible. Final review certifies fulfilled OpenSpec
requirements; authentication/import/scanner cutover gates remain separate.
