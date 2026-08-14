# Portable data format

MedTracker uses portable identifiers when data moves between clients or installations. Portable payloads never include Rails numeric IDs.

Use the OpenAPI contract for endpoint parameters and response envelopes. This guide defines the records inside snapshots and exports.

## Formats

| Format | Purpose |
|---|---|
| `medtracker.portable.v1` | Plaintext mobile snapshot and the data encrypted inside a migration bundle. |
| `medtracker.portable.v2` | Consistent sync snapshot with a cursor for later change-feed requests. |
| `medtracker.portable.encrypted.v1` | AES-256-GCM envelope used for portable export and import. |
| `medtracker.health_data.v1` | Plaintext health-data export. |
| `medtracker.backup.v1` | JSON file stored inside a ZIP backup. |

Every plaintext payload includes `scope`, `exported_at`, `source_instance_id`, and `records`. A sync snapshot also includes `cursor`.

## Security rules

Send the portable passphrase in `X-MedTracker-Portable-Passphrase`. Do not put it in a URL or JSON body.

The encrypted envelope identifies the cipher and key derivation function. It also contains a salt, plaintext checksum, and authenticated ciphertext. Treat the complete envelope as sensitive health data even though its record values are encrypted.

Mobile and sync snapshots are plaintext over the authenticated HTTPS connection. Do not log or cache their response bodies.

## Common record fields

Every record includes these fields:

| Field | Meaning |
|---|---|
| `portable_id` | Stable record identity across installations. |
| `updated_at` | Last update time in ISO 8601 format. |
| `etag` | Version used for conflict checks and sync batch preconditions. |

Nullable relationship fields contain a portable ID or `null`. Collection relationships contain arrays of portable IDs.

## Record collections

### People

People add `name`, `email`, `date_of_birth`, `person_type`, `has_capacity`, `location_portable_ids`, and `notification_preference_portable_id`.

### Locations

Locations add `name` and `description`.

### Medications

Medications add:

- `location_portable_id`, `name`, `friendly_name`, `category`, and `description`;
- `dose_amount`, `dose_unit`, and `default_schedule_type`;
- `current_supply` and `reorder_threshold`;
- `barcode`, `dmd_code`, `dmd_system`, and `dmd_concept_class`.

### Dosage options

Dosage options add:

- `medication_portable_id`, `amount`, `unit`, `frequency`, and `description`;
- `default_for_adults` and `default_for_children`;
- `default_max_daily_doses`, `default_min_hours_between_doses`, and `default_dose_cycle`;
- `current_supply` and `reorder_threshold`.

### Schedules

Schedules add:

- `source_dosage_option_portable_id` and `retired_at`;
- `person_portable_id` and `medication_portable_id`;
- `dose_amount`, `dose_unit`, `frequency`, and `dose_cycle`;
- `max_daily_doses` and `min_hours_between_doses`;
- `schedule_type`, `schedule_config`, `start_date`, and `end_date`;
- `active` and `notes`.

### Person medications

Direct person-medication assignments add:

- `source_dosage_option_portable_id` and `retired_at`;
- `person_portable_id` and `medication_portable_id`;
- `dose_amount`, `dose_unit`, and `dose_cycle`;
- `max_daily_doses` and `min_hours_between_doses`;
- `administration_kind`, `active`, `notes`, and `position`.

### Medication takes

Dose records add `client_uuid`, `source_type`, `source_portable_id`, `taken_at`, `dose_amount`, `dose_unit`, `taken_from_medication_portable_id`, and `taken_from_location_portable_id`.

`source_type` is `schedule` or `person_medication`. Medication takes are immutable after import.

### Notification preferences

Notification preferences add `person_portable_id`, `enabled`, `dose_due_enabled`, `missed_dose_enabled`, `low_stock_enabled`, `private_text_enabled`, `morning_time`, `afternoon_time`, `evening_time`, and `night_time`.

### Health events

Health events add `person_portable_id`, `event_kind`, `severity`, `title`, `notes`, `started_on`, `ended_on`, and `medication_portable_ids`.

The encrypted migration payload does not include health events. Mobile, sync, and household health-data snapshots include them.

## Import behaviour

Run the dry-run endpoint before applying an import. It returns record counts, conflicts, and validation errors without writing data.

Imports reject unknown record collections, Rails numeric IDs, invalid capacity rules, and missing portable relationships. A conflict identifies the record collection and incoming portable ID. It also reports the conflicting field and existing portable ID.

An applied import is transactional. If any record fails, MedTracker does not keep a partial import.

## Incremental sync

Start with `GET /sync/snapshot`. Store its cursor and each record ETag. Use the cursor with `GET /sync/changes` to read later changes and tombstones.

Send local writes to `POST /sync/batches`. Update and delete operations need the latest ETag in `if_match`. Medication-take creation uses `client_uuid` for idempotency. A stale ETag returns a sync conflict, and the complete batch rolls back.
