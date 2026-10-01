# Bounded D/E/F preparation and baseline audit classification

Read-only source inspection, 1 October 2026. No product/test edits, dependency
changes, runtime commands, builds or Git mutations. Only this report is written.
Serena instructions and Ruby/Rust review guidance were read. References below
use repository-relative paths and one-based line numbers. Current source was
checked; baseline runtime results are the earlier recorded differential evidence,
not a new run by this scout.

## D: dosage options — ready for a bounded editor brief

- Public collection `/households/{household_id}/dosage_options`: GET and POST.
  Member `/{id}`: GET, PATCH and PUT. There is no DELETE. Rails routes:
  `config/routes.rb:56`; OpenAPI: `docs/api/openapi.v1.yaml:1767–1940`.
- Request envelope `dosage_option`. Create requires `medication_id` identifier
  string; positive exact decimal-string `amount`; nonempty `unit` and `frequency`;
  positive integer `default_max_daily_doses`; nonnegative decimal-string
  `default_min_hours_between_doses`; `default_dose_cycle` daily/weekly/monthly.
  Optional description, adult/child default booleans and nullable decimal-string
  current_supply/reorder_threshold. IDs must not be JSON numbers. The dosage API
  permits custom nonempty units; do not impose scalar medication's unit whitelist.
- `rust/api/src/dosage_options.rs:79–219` enforces storage precision; stock fits
  eight integer digits/two decimals, minimum hours three integer digits/one
  decimal. Positive amount, nonnegative stock/threshold; null supply means
  untracked, zero means tracked empty. Preserve rejected draft text exactly.
- Writes require household owner/administrator through `write_context:283`.
  Parent medication must be in household. Browser nesting must additionally
  compare option.medication_id with the selected parent, rather than accepting
  any household option. Reads use parent visibility; never infer management
  permission from medication creation's broader manage-grant capability.
- Existing update takes option ETag and rejects stale supplied If-Match with 409,
  but **omission is currently accepted by the API** (`dosage_options.rs:1149`).
  Browser 428 for missing/blank ETag therefore needs its own explicit boundary.
  Submit the original tag; do not refresh/adopt it before the write.
- Creates lock household and parent; updates additionally lock option. Creating
  the first option sets parent dose_amount null even for an untracked option.
  Tracked supply aggregates parent supply and thresholds in the same transaction;
  errors roll back option and parent. Parent and option versions/sync events share
  request ID (`:973–1079`, `:1239–1304`). An untracked metadata-only option update
  does **not** currently update the parent version; parent change is conditional
  on inventory sync. Clarify intended parent-version acceptance before promising
  every option edit invalidates scalar parent forms.
- Adult/child defaults have uniqueness protection, but general duplicate options
  do not. Standalone option create/update has no mutation-idempotency lookup/store.
  Sync batches have a separate replay path (`:446–725`). A browser retry must not
  be described as safe-once until an actual guard is implemented and proved.
- Existing focused contracts: `openapi_dosage_options.rs:599` atomic parent/audit/
  sync effects; `:716` rejected quantities/defaults leave state unchanged;
  `:1131` invalid precision; `dosage_health.rs` immediate dose behaviour.

Suggested first D boundary: list/add/edit existing option-mode medication using
focused private adapter/renderer modules, fresh synthetic fixture medication and
retained custom units. Cover multiple tracked/untracked options, invalid/default
conflict, stale/missing browser ETag, foreign option-parent mismatch, CSRF and
immediate dose/stock read-back. Scalar-to-options conversion needs an explicit
reviewed transition because the parent scalar dose is cleared.

## E: stock semantics and gaps

Routes under `/households/{household_id}/medications/{id}`:

| Action | Actual request and behaviour |
| --- | --- |
| PATCH `/adjust_inventory` | `adjustment: {new_quantity: "…", reason: "…"}`. Absolute nonnegative stock, including zero. Reason is an optional string. Manager only; lock/re-authenticate then lock medication. |
| GET/POST `/stock_removals` | `stock_removal: {quantity, reason, submission_id, note?, dosage_id?}`. Positive decimal string, at most two decimals; lowercase UUID submission_id. Optional selected tracked dosage stock. |
| PATCH `/mark_as_ordered` | Optional `order_details: {supplier?, quantity?, expected_arrival_on?}`. Sets ordered status/timestamp/details. |
| PATCH `/mark_as_received` | Empty object only. Sets received status/reordered_at. **Does not add stock or accept a receipt quantity.** |

Sources: `config/routes.rb:48–54`, OpenAPI schemas `:6541`, `:7970`,
`rust/api/src/medication_management/inventory.rs`, `reorder.rs:78,169–183`,
`stock_removals.rs:193–274,321–536`.

Removal reasons are exactly dropped, damaged, expired, discarded, lost,
transferred_out, other. Note limit is 1000. Quantity cannot be zero, negative,
numeric JSON or exceed selected supply. Removal records versions and sync events,
uses submission ID/payload matching for replay and rejects changed payloads. With
tracked dosage inventory, selected option decrements and parent totals refresh.
Source arithmetic remains Decimal. Option rows with null stock are distinct from
empty tracked rows.

Adjust and order actions reauthenticate under the household lock, but their source
does not contain mutation-idempotency lookup/store or an If-Match check. Repeating
an absolute adjustment does not double-decrement, though repeated audit events are
possible. Do not claim all stock writes have equivalent replay semantics. Inventory
adjustment updates the medication row directly; review tracked-option mode before
offering parent adjustment that could disagree with option totals.

Partial receipt is an actual missing capability of the present received endpoint,
not a hidden existing request field. Delivering quantity receipt needs a deliberate
API/contract decision. Keep order status separate from supply changes in the UI.
Focused existing contracts are `openapi_stock_workflows.rs` and
`medication_stock.rs`; dose consumption remains a required regression.

## F: direct assignment before seven schedule families

`/households/{household_id}/person_medications` supports GET/POST; `/{id}` supports
GET/PATCH/PUT plus `/pause`, `/resume`, `/reorder`. There is **no public DELETE** in
Rails or Rust routing/OpenAPI. Sync-batch `delete` retires the record, sets inactive
and emits deletion events (`person_medication_writes.rs:1381–1430`). Browser remove
cannot be implemented by pretending a public delete exists.

Create envelope `person_medication` requires person_id and medication_id identifier
strings. Optional source_dosage_option_id, exact dose_amount/dose_unit snapshot,
administration_kind routine/as_needed, notes, max_daily_doses, minimum hours and
dose_cycle daily/weekly/monthly. Minimum hours are positive whole-number decimal
strings because direct assignment storage uses integer hours; dose fits two
decimals. Updating person_id may only repeat the existing person, never move the
assignment. Source option must belong to medicine and match the snapshot. Defaults
can resolve the medicine's matching dosage option (`:523–583`).

Management uses current unrevoked/unexpired **person manage grant**, not a blanket
household manager rule (`person_medication_writes.rs:297`). Visibility of person,
medicine and selected option must all hold. Mutation checks occur after household
lock/re-authentication. Updates require original If-Match. Header idempotency guards
retain payload conflict detection and reauthorise replay (`:463–517,839–902`).
Contracts: `openapi_person_medication_writes.rs`, particularly `:542` default option
resolution and `:1217` precision/idempotent replay.

Seven actual schedule types, shared by `app/models/schedule.rb:29` and OpenAPI
`:7132`: **daily, multiple_daily, weekly, specific_dates, prn, tapering,
every_other_day**. Monthly is a dose cycle, not an eighth schedule type.

Schedule fields include person/medication/option IDs, snapshot dose/unit, start_date,
end_date, active, frequency, notes, limits and schedule_config. Config exposes times,
weekdays, dates, as_needed and taper_steps (`OpenAPI:7404`). Each taper step needs
start/end dates and may override amount/dose_amount, unit/dose_unit, limits and
times. HH:MM times and canonical/abbreviated/numeric weekdays are validated.
Rails applies date range before weekly weekday selection, specific dates,
every-other-day parity from start or taper step selection (`Schedule/applies_on?`).
PRN has no expected routine doses. Families need actual effective-date/overlap,
timezone/DST and pause/resume evidence; type enumeration alone is not acceptance.
Focused contract files: `schedules.rs`, `openapi_schedule_writes.rs`; implementation
`schedule_writes.rs` and authorised projections `read_resources/schedules.rs`.

## #2347: classification, without weakening expectations

Earlier candidate and clean pre-refactor baseline both recorded 56/16, 7/2 and
21/2. Exact evidence is retained in `refactor-20261001-review.md:205–289`.

| Existing failures | Classification and concrete next action |
| --- | --- |
| Four dosage helper setup failures | Fixture drift: direct SQL insert omits required default_dose_cycle in `dose_mode_transition_api.rs:41` / `management_sync_events_api.rs:128`. Repair realistic helpers, then run intended assertions; not a dosage product verdict. |
| Seven medication timestamp failures | Expectation drift: `medication_stock.rs:57` fixes length at 20, but deliberate microsecond RFC3339/ETag timestamps have length 27. Assert valid required precision/format rather than truncate product timestamps. |
| Two source stock ordering/pause failures | Unresolved product-versus-contract question. `source_capabilities_api.rs:376` requires location ordering whereas current projection orders IDs; `:293` expects paused eligibility empty although stock loader projects permitted matching supply. Neither mismatch establishes that test or product is correct. Decide intended display ordering/pause eligibility and retain dose submission's independent pause guard. |
| One People pagination failure | Product/contract parity gap requiring decision: `web_reads_api.rs:334` expects 200 clamp-to-100 for per_page=999; current shared validator returns 422. Do not call harmless fixture drift. |
| Two viewer dashboard failures | Product failure: valid limited-view login redirects to dashboard 503 before permission assertions. Slice C must remove optional-profile dependency, without adding own-person grant. Linked #2345. |
| Two Medication exact-key failures | Expectation drift: optional authoritative barcode/friendly_name/warnings were added to serializer before refactor; static key list at `openapi_medications.rs:58` omits them. Preserve useful edit fields. |
| Two read-cache exact-header failures | Expectation drift: current private, no-store versus exact no-store at `openapi_read_completion.rs:110,218`. Preserve private response protection and test directives semantically. |

The two source-projection and one pagination cases remain undecided; the broad
audits remain RED. Source-only review cannot certify fixes or runtime parity.

## Build-lane warning and handoff

Named Tasks `api:openapi-dosages-acceptance`, `api:openapi-stock-workflows-acceptance`,
`api:openapi-person-medication-writes-acceptance` and `api:openapi-schedule-writes-acceptance`
currently invoke `run.fish rails …` (`rust/api/Taskfile.yml:259,280,393,805`). They
are Rails reference acceptance selectors, not evidence of a Rust run by name
alone. Coordinator/Luna must select the existing Rust-target execution explicitly
and record target/fixture/digest. No new test execution was attempted here.

Scout preparation complete. D/E/F writers retain product ownership; coordinator
owns public API decisions, route wiring and the unresolved parity contracts.
