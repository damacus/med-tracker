## Purpose

Defines authorized, atomic, and PHI-safe creation of idempotent medication takes through the sync API.

## Requirements

### Requirement: Sync batches accept medication-take creation
The system SHALL accept a sync batch operation whose `action` is `create` and whose `resource_type` is `medication_take`. The operation SHALL omit update preconditions and SHALL provide its dose input through `attributes`, including a non-blank `client_uuid`, `source_type`, `source_id`, and `taken_at`.

#### Scenario: Create a queued medication take
- **GIVEN** an authenticated household client can view an active medication source and has `record` access for its person
- **WHEN** the client submits a valid `create` operation for `medication_take`
- **THEN** the batch succeeds with HTTP 201 and identifies the newly created medication take in that operation's result
- **AND** the operation result reports `replayed` as false

#### Scenario: Reject a missing idempotency key
- **GIVEN** a medication-take create operation has no non-blank `client_uuid`
- **WHEN** the client submits the sync batch
- **THEN** the system rejects the batch with HTTP 422 and error code `medication_take_invalid`
- **AND** no operation in the batch remains applied

#### Scenario: Reject mutation of medication-take history
- **GIVEN** a sync operation targets `medication_take`
- **WHEN** its action is `update` or `delete`
- **THEN** the system rejects the batch with HTTP 422 and error code `sync_operation_unsupported`
- **AND** the existing medication take remains unchanged

### Requirement: Queued medication takes are idempotent
The system SHALL treat `client_uuid` as the identity of a queued medication take. A replay that is authorized to access the matching take SHALL return that take without recording another administration or repeating any side effect.

#### Scenario: Replay an existing queued take
- **GIVEN** an authorized client has already created a medication take with a `client_uuid`
- **WHEN** the client submits another medication-take create operation with that `client_uuid`
- **THEN** the batch succeeds and identifies the original medication take
- **AND** the operation result reports `replayed` as true
- **AND** no second medication take, stock decrement, audit version, or sync change record is created

#### Scenario: Concurrent retries converge on one take
- **GIVEN** two authorized batches concurrently submit the same new `client_uuid`
- **WHEN** both batches are processed
- **THEN** both successful operation results identify the same medication take
- **AND** exactly one medication take and one set of stock, audit, and sync side effects exists

#### Scenario: A hidden idempotency key is not disclosed
- **GIVEN** a submitted `client_uuid` conflicts with a medication take the client is not authorized to access
- **WHEN** the batch cannot replay the hidden take
- **THEN** the system rejects the batch with HTTP 409 and error code `idempotency_key_unavailable`
- **AND** the response does not disclose the take, person, medication, source, or household

### Requirement: Batch recording reuses medication-administration rules
The system SHALL record a queued take only through the normal medication-administration boundary and SHALL enforce the same source visibility, person-level `record` grant, timing, dose, stock-source, inventory, immutable-history, audit, change-record, tenant, and household rules as direct medication-take creation.

#### Scenario: Record a valid dose through the canonical boundary
- **GIVEN** a visible source, an applicable person `record` grant, and a dose that satisfies all medication rules
- **WHEN** the queued take is processed
- **THEN** the persisted take contains the normal source, timing, dose, stock-source, household, user, and client UUID data
- **AND** inventory, audit history, and sync change records match a dose recorded through the direct endpoint

#### Scenario: Deny a visible source without record authority
- **GIVEN** the client can view a medication source but lacks `record` access for its person
- **WHEN** the client submits a queued take for that source
- **THEN** the system rejects the whole batch with HTTP 403 and error code `forbidden`
- **AND** no medication-take or inventory side effect remains

#### Scenario: Hide a stale or cross-household source
- **GIVEN** the source is stale, outside the active household, or not visible to the client
- **WHEN** the client submits a queued take referencing it
- **THEN** the system rejects the whole batch with HTTP 404 and error code `not_found`
- **AND** the response does not reveal whether the source exists

#### Scenario: Reject unavailable stock
- **GIVEN** the requested dose cannot be fulfilled by the selected tracked stock source
- **WHEN** the client submits the queued take
- **THEN** the system rejects the whole batch with HTTP 422 and error code `medication_stock_unavailable`
- **AND** no medication-take or inventory side effect remains

#### Scenario: Reject a timing conflict
- **GIVEN** the requested dose violates a cooldown, daily limit, overlapping administration, paused source, or other timing rule
- **WHEN** the client submits the queued take
- **THEN** the system rejects the whole batch with HTTP 422 and error code `medication_timing_conflict`
- **AND** no medication-take or inventory side effect remains

### Requirement: Medication-take batches remain atomic
The system SHALL apply medication-take creates and their sibling sync operations in one transaction. Any failed operation SHALL roll back all records, inventory mutations, audit versions, tombstones, and sync change records produced by the batch.

#### Scenario: A failed queued take rolls back earlier operations
- **GIVEN** an earlier operation in a batch is valid and a later medication-take create is invalid or unauthorized
- **WHEN** the batch is processed
- **THEN** the system returns the medication-take failure
- **AND** the earlier operation and every medication-take side effect are rolled back

#### Scenario: A later failure rolls back a queued take
- **GIVEN** a medication-take create is valid and a later sibling operation fails
- **WHEN** the batch is processed
- **THEN** the system returns the later operation's failure
- **AND** the take, stock decrement, audit version, and sync change record are rolled back

### Requirement: Medication-take batch failures are stable and PHI-safe
The system SHALL use machine-readable error codes for medication-take batch failures and SHALL NOT echo submitted identifiers, medication names, person details, dose values, clinical notes, or data from an inaccessible record.

#### Scenario: Reject an unsupported action safely
- **GIVEN** a medication-take operation requests an action other than `create`
- **WHEN** the batch is rejected
- **THEN** the response uses error code `sync_operation_unsupported`
- **AND** the response may identify the zero-based operation index but contains no PHI

#### Scenario: Reject invalid dose input safely
- **GIVEN** a medication-take operation contains an invalid timestamp, dose, source type, or stock selection
- **WHEN** the batch is rejected
- **THEN** the response uses a stable public error code for the failure category
- **AND** the response contains no submitted clinical value or inaccessible record data
