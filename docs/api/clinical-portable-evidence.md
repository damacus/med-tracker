# Clinical portable transfer evidence

This is the bounded compatibility evidence for [#2702](https://github.com/damacus/med-tracker/issues/2702).
It covers the retained Rails serializer against fresh Rust, using synthetic
records only. It is not a live-data migration procedure or proof of complete
Rails database export.

## Household ZIP acceptance status

The newer 0.5 household ZIP journey is described in the
[transfer guide](clinical-transfer-preparation.md#household-zip-import).
On 7 November 2026, local checks exercised it with synthetic ZIP data, not an
export from a hosted 0.5 installation:

- The final combined integration retry passed 61 of 61 cases: 36 new import
  cases and 25 affected older portable-import cases. These cover RLS, consent
  and withdrawal, self preview, pagination, create-only native values and
  foreign keys, junctions, canonical history, stock, replay, concurrency
  and rollback after a late write failure. See the
  `tests/care_api/data_import_authority.rs`,
  `tests/care_api/data_import_atomicity.rs` and
  `tests/care_api/data_import_report_access.rs`.
- The whole-application browser suite passed 75 journeys, including the
  Administration import journeys in `tests/browser/administration_polish.rs`.
  Its Ready and Completed pages were captured on
  [desktop at Ready](../screenshots/clinical-data-import/ready-desktop.png),
  [mobile at Ready](../screenshots/clinical-data-import/ready-mobile.png),
  [desktop at Completed](../screenshots/clinical-data-import/completed-desktop.png)
  and [mobile at Completed](../screenshots/clinical-data-import/completed-mobile.png).
  The journey checked that both reports fit a 390 px mobile viewport.
- Global Rust unit tests (359), JavaScript tests (34) and documentation checks
  passed. The client-tools CI suite passed 33 tests. These are suite-wide
  counts, not counts of import-specific tests.
- Local synthetic canary boot and replay passed two cases; a separate ignored
  template-concurrency case passed once.

The first full native integration run completed 1,241 cases: 1,232 passed and
nine new cases failed. After fixes, the combined 61-case retry above passed.
This is not a clean full-suite rerun or a single passing `task ci` invocation.
The final strict lint rerun passed. After the final index repair, all six
targeted import browser journeys also passed.

The older fixture evidence below was documented at revision
`95a5e0722eab755d3d953c683bf68f254f51e1ee` and remains limited to its named
tests and synthetic inputs. Neither the older fixtures nor the new local checks
prove a hosted acceptance case or universal compatibility.

The isolated Home Ops configuration in
[PR #4716](https://github.com/damacus/home-ops/pull/4716) was reviewed with
Flux suspended, the web deployment at zero replicas and the initialisation
job suspended. It is configuration evidence, not a running source. This work
has not provisioned, seeded or exported a real hosted 0.5 canary. Independent
canary credentials, provisioning, seed and export remain pending. No production
deployment is authorised, and no hosted address or operator access has been
verified.

The pending canary acceptance must capture and compare:

1. The expected household records and relationships from the reviewed seed
   specification, including values and dates at the actual seed time.
2. The actual 0.5 export: archive identity and checksum, inventory, records,
   relationships and omissions.
3. The reviewed 0.6 preview: per-person and shared expected outcomes,
   dependencies, consent decisions and exact omission acknowledgements.
4. The completed 0.6 report and independently read destination records and
   verified relationships, including replay and historical stock behaviour.

Record discrepancies at each stage and preserve the source. Acceptance also
needs authorised operator access to a separately provisioned source and a fresh
0.6 destination, followed by a visual check of imported people, medicines and
everyday care workflows. Until those steps occur, there is no real-canary or
hosted acceptance result.

## Fixture provenance

The fixtures live in `tests/fixtures/portable_rails/`. They were produced by a
synthetic harness that loaded the retained Rails exporter, record/event/outcome
serializers, ETag and Encryptor without booting Rails, loading application models
or connecting to a database. Its exporter subclass supplied an in-memory record
graph and suppressed export audit writes. The production collection selection,
record serializers and ActiveSupport JSON/encryption ran unchanged.

The harness created an uninitialised configuration-only Rails application and
loaded the framework defaults selected in retained `rails/config/application.rb`.
It applied the same message serializer and metadata settings as the Rails
ActiveSupport initialisers. Standalone ActiveSupport defaults to Marshal;
using that default would have produced an incorrect fixture for this Rails
application. See the [Rails configuration source](https://github.com/rails/rails/blob/v8.1.4/railties/lib/rails/application/configuration.rb)
and [ActiveSupport initialisers](https://github.com/rails/rails/blob/v8.1.4/activesupport/lib/active_support/railtie.rb).

The fixtures are frozen: the retained Rails tree is being archived and the
generator is retired, so they are committed test data, not reproducible output. Their
recorded provenance is Ruby 4.0.7 with ActiveSupport 8.1.4 under Rails 8.0
framework defaults. The passphrase is the deliberately public synthetic test
value `synthetic portable secret`. Never use it for actual records.

```fish
rtk task test:integration TARGET=care_api FILTER=rails_serializer_
```

| Fixture | Actual retained Rails path | Contents / expected Rust handling |
|---|---|---|
| `migration-v1` | `Exporter#payload` and `Encryptor.encrypt` | Eight base collections; empty person-location links. |
| `migration-v2` | `Exporter#v2_payload` and `Encryptor.encrypt` | Base collections plus closed pause and saved not-taken outcome; no health events. |
| `household-v2` | `Exporter#household_payload` and `Encryptor.encrypt` | All eleven collections, including linked health history. The envelope wraps the household payload explicitly; this is not `Exporter#call` or direct ZIP restore. |
| `linked-migration-v2` | Same v2 path with a synthetic person-location association | Rejected by Rust dry-run and apply before domain writes. |
| `historical-location-v2` | Household v2 with stock moved since a historical take | Decrypts successfully; dry-run reports the location mismatch and apply rejects without domain writes. This is a retained v1/v2 compatibility gap, not discarded or rewritten history. |

Each row has a plaintext `.json` and Rails-produced `.encrypted.json`.
The additional `prepared-history-v2` household fixture and HostedExport manifest
exercise the explicit [preparation path](clinical-transfer-preparation.md), with
different current and historical stock labels and all eleven collections. Its
identities are separate from the rejection fixtures. Hosted manifests and archive
entry serialization use the retained pure builders without running database
selection or export authorisation.
The fixtures contain two storage labels, fractional medication/dosage stock,
one dependent adult, a schedule,
routine assignment, a historical take, a closed pause with unavailable original
actors, a not-taken outcome, notification preferences and a health event.
Accepted examples have no stock move. The separate historical-location fixture
keeps distinct current and taken-from labels and proves the current v2 rejection.

## Collection and field matrix

All rows include `portable_id`, `updated_at` and `etag`. Portable identity is
preserved. Rust creates destination update/version metadata and ETags: those
are not cross-installation clinical provenance and must not be copied as local
concurrency tokens. Decimal strings are compared numerically, without losing
fractional stock; clinical timestamps are compared at microsecond precision.

The fields below describe the retained serializers and the older portable
import contract exercised by these fixtures. Acceptance of a field name alone
does not establish all values, new ZIP behaviour or export-selection completeness.

| Collection | Fields beyond common identity | Mapping / boundary |
|---|---|---|
| `people` | `name`, `email`, `date_of_birth`, `person_type`, `has_capacity`, `location_portable_ids`, `notification_preference_portable_id` | New identity is linked to a fresh household participant. Empty obsolete links are accepted in v1/v2; non-empty links rejected. New dependents need an explicit current guardian declaration. No account or credential import. |
| `locations` | `name`, `description` | Storage labels; no person-location membership is created. |
| `medications` | `location_portable_id`, `name`, `friendly_name`, `category`, `description`, `dose_amount`, `dose_unit`, `default_schedule_type`, `current_supply`, `reorder_threshold`, `barcode`, `dmd_code`, `dmd_system`, `dmd_concept_class` | Location portable reference resolves locally; recorded stock is restored without administering a dose. |
| `dosage_options` | `medication_portable_id`, `amount`, `unit`, `frequency`, `description`, `default_for_adults`, `default_for_children`, `default_max_daily_doses`, `default_min_hours_between_doses`, `default_dose_cycle`, `current_supply`, `reorder_threshold` | Parent portable reference resolves locally; stock and defaults retained. |
| `schedules` | `source_dosage_option_portable_id`, `retired_at`, `person_portable_id`, `medication_portable_id`, `dose_amount`, `dose_unit`, `frequency`, `dose_cycle`, `max_daily_doses`, `min_hours_between_doses`, `schedule_type`, `schedule_config`, `start_date`, `end_date`, `active`, `notes` | References resolve within imported graph/destination; pause history determines final active state. This fixture exercises one daily configuration, not every scheduling variant. |
| `person_medications` | `source_dosage_option_portable_id`, `retired_at`, `person_portable_id`, `medication_portable_id`, `dose_amount`, `dose_unit`, `dose_cycle`, `max_daily_doses`, `min_hours_between_doses`, `administration_kind`, `active`, `notes`, `position` | Routine assignment and source dosage reference retained; as-needed/retired variants need separate evidence. |
| `medication_takes` | `client_uuid`, `source_type`, `source_portable_id`, `taken_at`, `dose_amount`, `dose_unit`, `taken_from_medication_portable_id`, `taken_from_location_portable_id` | Original source/time and same-location provenance restore without stock deduction. V1/v2 reject a taken-from label different from current medicine storage; Rust v3 allows that distinction. No automatic conversion is provided. |
| `notification_preferences` | `person_portable_id`, `enabled`, `dose_due_enabled`, `missed_dose_enabled`, `low_stock_enabled`, `private_text_enabled`, `morning_time`, `afternoon_time`, `evening_time`, `night_time` | Preference flags/times and person link restored; restore creates no reminders or mail jobs. |
| `medication_pause_periods` (v2+) | `source_type`, `source_portable_id`, `reason`, `note`, `started_at`, `ended_at`, `created_at`, `legacy_context`, `imported_context`, `recorded_by_person_portable_id`, `resumed_by_person_portable_id` | Original timing/context and unresolved actor portable references retained; `imported_context` becomes true. The importer has separate local audit attribution. |
| `dose_occurrences` (v2+) | `source_type`, `source_portable_id`, `window_starts_on`, `window_ends_on`, `position`, `scheduled_at`, `outcome`, `reason`, `note`, `resolved_at`, `medication_take_portable_id` | Saved outcome identity, window and resolution retained. Newly imported outcomes attribute the local restore to the importer; no administration is replayed. |
| `health_events` (v2+) | `person_portable_id`, `event_kind`, `severity`, `title`, `notes`, `started_on`, `ended_on`, `medication_portable_ids` | Household payload includes these; encrypted migration payload omits them. Do not claim a migration bundle is complete health-history backup. |

## Acceptance and remaining work

The acceptance module is `tests/care_api/portable_rails.rs`, registered beneath
`portable_imports`. It uses the full Loco test application and restricted runtime
credentials with disposable PostgreSQL. It decrypts Rails envelopes with Rust,
checks dry-run counts and no domain effects, rejects missing guardian declarations,
applies with an explicit declaration, replays without one, and reads the saved
graph back through the normal sync route. It compares emitted clinical fields,
stock balances and relationships, checks no person-location memberships, and
checks no reminder, delivery or queue-job side effects.
Storage labels are checked through typed persisted rows, because clinical sync
does not include an unused storage label merely because it was restored.

The moved-stock fixture is separate negative evidence. In
`src/models/care/portable_imports/preflight.rs`, the non-v3 branch compares each
take's historical label with the medicine's current label. Both labels are
present and valid in this synthetic Rails graph, but v2 is refused. Keeping the
failure visible is not proof of supported transfer for that history.

Supported format markers are v1/v2/v3. The retained Rails exporter supports only
v1/v2. V3 forbids the obsolete person-location field entirely. Do not relabel old
payloads or strip relationships automatically. Existing focused Rust tests cover
v3 export/import and obsolete-link rejection; they do not turn Rails v2 into v3.

**#2702 remains open** for the following evidence/decisions:

- A reviewed operator path for complete clinical transfer when Rails person-location
  links are non-empty, historical taken-from labels differ from current stock,
  or health events are absent from the chosen export. This older fixture
  evidence does not establish automatic data removal or ZIP restoration. The explicit
  [household preparation path](clinical-transfer-preparation.md) now addresses
  moved-stock history and included health events when obsolete links are empty;
  it deliberately rejects nonempty obsolete links and attachments.
- Database-backed Rails export selection, model validations, actual deployed Ruby/
  Rails dependency versions and every supported schedule/history variant. The
  in-memory fixture establishes serializer/envelope interoperability only.
- Integration with the ongoing person-owned care-sharing and protected restore
  provenance work. This slice changes no restore, sharing or schema implementation.
- Full access-denial/conflict coverage across existing/self/guardian subjects,
  open/legacy/retired history and taken outcomes with linked takes. Existing Rust
  tests are useful evidence but are not the same as Rails-produced fixtures.

Fresh accounts and authentication are configured separately. No Rails database
adoption, credential migration, reverse migration, live export or deployment is
included. Profile JSON/ZIP downloads remain a separate contract under #2513.
