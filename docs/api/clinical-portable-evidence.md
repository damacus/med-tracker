# Clinical portable transfer evidence

This is the bounded compatibility evidence for [#2702](https://github.com/damacus/med-tracker/issues/2702).
It covers the retained Rails serializer against fresh Rust, using synthetic
records only. It is not a live-data migration procedure or proof of complete
Rails database export.

## Reproduce the evidence

The fixtures live in `tests/fixtures/portable_rails/`. `generate.rb` loads the
retained Rails exporter, record/event/outcome serializers, ETag and Encryptor
without booting Rails, loading application models or connecting to a database.
Its exporter subclass supplies an in-memory record graph and suppresses export
audit writes. The production collection selection, record serializers and
ActiveSupport JSON/encryption run unchanged.

The harness creates an uninitialised configuration-only Rails application and
loads the framework defaults selected in retained `rails/config/application.rb`.
It applies the same message serializer and metadata settings as the Rails
ActiveSupport initialisers. Standalone ActiveSupport defaults to Marshal;
using that default would produce an incorrect fixture for this Rails application.
See the [Rails configuration source](https://github.com/rails/rails/blob/v8.1.4/railties/lib/rails/application/configuration.rb)
and [ActiveSupport initialisers](https://github.com/rails/rails/blob/v8.1.4/activesupport/lib/active_support/railtie.rb).

Use Ruby 3.2 or newer with the exact ActiveSupport and railties versions from
`rails/Gemfile.lock` already available. The generator records the
exact Ruby/ActiveSupport versions and SHA-256 of every loaded source in
`provenance.json`. The dependency is for regenerating/checking reference fixtures,
not a new Rust runtime or CI dependency. No gem installation is part of the task.

```fish
# Set RUBY to an existing compatible executable if the system Ruby is older.
env RUBY=/path/to/ruby rtk task test:portable-rails-fixtures
rtk task test:integration TARGET=care_api FILTER=rails_serializer_
```

To regenerate after reviewing a reference-source change, run
`ruby tests/fixtures/portable_rails/generate.rb` with that same runtime.
Encryption uses random salts/nonces, so regeneration changes ciphertext;
`--check` decrypts the committed envelope and compares its plaintext instead.
The passphrase is the deliberately public synthetic test value
`synthetic portable secret`. Never use it for actual records.

| Fixture | Actual retained Rails path | Contents / expected Rust handling |
|---|---|---|
| `migration-v1` | `Exporter#payload` and `Encryptor.encrypt` | Eight base collections; empty person-location links. |
| `migration-v2` | `Exporter#v2_payload` and `Encryptor.encrypt` | Base collections plus closed pause and saved not-taken outcome; no health events. |
| `household-v2` | `Exporter#household_payload` and `Encryptor.encrypt` | All eleven collections, including linked health history. The envelope wraps the household payload explicitly; this is not `Exporter#call` or direct ZIP restore. |
| `linked-migration-v2` | Same v2 path with a synthetic person-location association | Rejected by Rust dry-run and apply before domain writes. |
| `historical-location-v2` | Household v2 with stock moved since a historical take | Decrypts successfully; dry-run reports the location mismatch and apply rejects without domain writes. This is a retained v1/v2 compatibility gap, not discarded or rewritten history. |

Each row has a plaintext `.json` and Rails-produced `.encrypted.json`.
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

The fields below are emitted by the retained serializers and accepted by the
current import contract. Acceptance of a field name alone does not establish
all values or export-selection completeness.

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
  or health events are absent from the chosen export. No
  adapter, automatic data removal or ZIP restoration is delivered here.
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
