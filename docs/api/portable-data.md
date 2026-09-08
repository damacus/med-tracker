# Portable data format

MedTracker uses portable identifiers when data moves between clients or installations. Portable payloads never include Rails numeric IDs.

Use the OpenAPI contract for endpoint parameters and response envelopes. This guide defines the records inside snapshots and exports.

## Formats

| Format | Purpose |
|---|---|
| `medtracker.portable.v1` | Plaintext mobile snapshot and the data encrypted inside a migration bundle. |
| `medtracker.portable.v2` | Migration data with pause history and persisted dose outcomes, or a consistent sync snapshot with a cursor. |
| `medtracker.portable.encrypted.v1` | AES-256-GCM envelope used for portable export and import. |
| `medtracker.health_data.v1` | Plaintext health-data export. |
| `medtracker.backup.v1` | JSON file stored inside a ZIP backup. |

Every plaintext payload includes `scope`, `exported_at`, `source_instance_id`, and `records`. A sync snapshot also includes `cursor`.

Request `portable_format=medtracker.portable.v2` on the encrypted export endpoint to include saved dose outcomes. Omitting this parameter retains the v1 format and collection shape.

### Dose outcomes in v2 bundles

The `dose_occurrences` collection contains persisted outcomes only. Export does not generate open dose rows or change stock. Records follow their source schedules and assignments, including retained sources.

Each row includes the common portable identity and version fields, plus `source_type`, `source_portable_id`, `window_starts_on`, `position`, nullable `scheduled_at`, `outcome`, nullable `reason`, nullable `note`, nullable `resolved_at`, and nullable `medication_take_portable_id`.

References use portable IDs. Local membership IDs and signed API occurrence keys are not portable. The destination derives its own opaque keys from the preserved source, window and position. V1 bundles do not contain this collection.

## Security rules

Send the portable passphrase in `X-MedTracker-Portable-Passphrase`. Do not put it in a URL or JSON body.

Request `portable_export?version=2` to include pause history in an encrypted migration
bundle. The default remains version 1. Both versions can be imported; v1 collections
remain unchanged. Sync v2 snapshots include pause periods automatically.

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

### Medication pause periods (v2)

`medication_pause_periods` records contain `source_type` (`schedule` or
`person_medication`), `source_portable_id`, `reason`, optional `note`, `started_at`,
`ended_at`, `created_at`, `legacy_context`, and `imported_context`, plus the common
portable identity fields. An open period has no end. Legacy context uses
`reason_not_recorded` and may have an unknown start. Its creation time is preserved
so reports do not invent an earlier pause boundary.

`recorded_by_person_portable_id` and `resumed_by_person_portable_id` identify actors
where their membership has a person. Import resolves an actor only when exactly one
membership in the destination household matches. Otherwise the original reference is
retained and the actor is unavailable. Imported records have `imported_context: true`;
the import audit event identifies the importer separately from the original actors.

Import restores records without calling pause or resume actions. It preserves original
context, rejects conflicting history or multiple open periods, and derives each
affected source's active state from its final open period. Any failure rolls back the
entire import. Reimporting the same periods does not create duplicates or change stock.
An inactive current source without an open period is rejected as incomplete history.
Existing native pause records remain unchanged on reimport, including their provenance
and closing actor. Only a previously imported open period can advance to a closed period
through import; its original recording context remains immutable.

### Notification preferences

Notification preferences add `person_portable_id`, `enabled`, `dose_due_enabled`, `missed_dose_enabled`, `low_stock_enabled`, `private_text_enabled`, `morning_time`, `afternoon_time`, `evening_time`, and `night_time`.

### Health events

Health events add `person_portable_id`, `event_kind`, `severity`, `title`, `notes`, `started_on`, `ended_on`, and `medication_portable_ids`.

The encrypted migration payload does not include health events. Mobile, sync, and household health-data snapshots include them.

## Import behaviour

Run the dry-run endpoint before applying an import. It returns record counts, conflicts, and validation errors without writing data.

Imports reject unknown record collections, Rails numeric IDs, invalid capacity rules, and missing portable relationships. A conflict identifies the record collection and incoming portable ID. It also reports the conflicting field and existing portable ID.

An applied import is transactional. If any record fails, MedTracker does not keep a partial import.

Imports accept v1 and v2 plaintext inside the same encrypted envelope. V2 can restore saved dose outcomes and health events. Outcome references must resolve within the destination household or the imported graph. A linked take must match the source and day. Duplicate dose slots and changes to existing outcome history are rejected during dry run.

Restoring outcomes does not administer doses or deduct stock. An unchanged outcome can be imported again. The original resolution time is retained; the importing membership is recorded as the local actor for a newly restored outcome. Existing outcomes retain their local actor. Members need a current manage grant for every affected person.

Existing medication takes remain authoritative when the same portable ID is imported again. Validation uses their stored source and timestamp. Hosted household export archives include the v2 outcome collection in their portable payload.

## Incremental sync

Start with `GET /sync/snapshot`. Store its cursor and each record ETag. Use the cursor with `GET /sync/changes` to read later changes and tombstones.

Send local writes to `POST /sync/batches`. Update and delete operations need the latest ETag in `if_match`. Medication-take creation uses `client_uuid` for idempotency. A stale ETag returns a sync conflict, and the complete batch rolls back.

### Queued schedules and medication assignments

The batch endpoint accepts `create`, `update`, and `delete` for `schedule` and `person_medication`. The caller needs manage access to the person. Every referenced person, medicine and dosage option must be visible in the active household.

Both resources accept `person_id`, `medication_id`, `dose_amount`, `dose_unit`, `source_dosage_option_id`, `notes`, `max_daily_doses`, `min_hours_between_doses`, and `dose_cycle`. Schedules also accept `frequency`, `start_date`, `end_date`, `schedule_type`, and `schedule_config`. Medication assignments also accept `administration_kind`. These fields follow the same validation rules as the direct API. Updates cannot change `person_id`.

Use portable IDs or string database IDs for references. Decimal values, including `dose_amount`, `min_hours_between_doses`, and amounts inside a taper schedule, must be JSON strings. Portable IDs, household ownership, audit fields, active state and retirement timestamps are server-controlled.

For example, send the following body to `POST /api/v1/households/{household_id}/sync/batches` with bearer authentication and an `Idempotency-Key` header:

```json
{
  "batch": {
    "operations": [
      {
        "action": "create",
        "resource_type": "schedule",
        "attributes": {
          "person_id": "<person-portable-id>",
          "medication_id": "<medication-portable-id>",
          "dose_amount": "1",
          "dose_unit": "tablet",
          "start_date": "2026-09-05",
          "end_date": "2026-10-05",
          "schedule_type": "daily"
        }
      }
    ]
  }
}
```

Each successful result contains its operation index, action, record type and server-assigned `record_portable_id`. Create and update results also contain an `etag`. Use the latest ETag as the exact `if_match` string on the next update or delete, with the portable ID in `id`. A missing version returns `428 precondition_required`; a stale version returns `409 sync_conflict`.

Delete retires the schedule or assignment. It no longer appears in active lists, but its past doses and pause history remain unchanged. The change feed records the retirement and a deletion marker for the same portable ID. Retrying a successful delete with the same idempotency key replays the result. A new request for a retired item returns not found. Reorder and reactivation are not batch actions.

### Pause periods in sync batches

Use `resource_type: medication_pause_period` with `action: create` to pause a
source. Attributes are `source_type` (`schedule` or `person_medication`), portable
`source_id`, a supported `reason`, and optional `note`. Do not send `id` or `if_match`
for creation. The server records the effective start and actor.

Use `action: close`, the period's portable `id`, its latest ETag in `if_match`, and
empty attributes to resume. The server records the effective end and actor. Period
update and delete operations are unsupported. Successful results include `index`,
`action`, `record_type: MedicationPausePeriod`, `record_portable_id`, `etag`, and
`replayed`. Failed operations roll back the complete batch.

Persist one `Idempotency-Key` with each queued batch. If the connection drops, resend the identical body and key. The existing response is replayed without repeating writes, stock changes, audit entries or deletion markers. Changing the body while reusing the key returns `409 idempotency_key_reused`. The existing replay window is 24 hours; after that window, reconcile with the server before sending a new request. Requests without a key do not receive this batch-level replay protection.

Operations run in their submitted order. A failed operation rolls back every domain write in the batch, including earlier dose recording, stock changes and associated audit and sync records. Error responses do not include submitted clinical values. References to results of earlier operations are not supported: use the returned portable IDs in a later batch. A corrected batch should use a new idempotency key.
