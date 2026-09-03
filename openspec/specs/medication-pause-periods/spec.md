## Purpose

Defines attributable pause periods for scheduled and direct medication sources while preserving dose expectations, history, authorization, and data portability.

## Requirements

### Requirement: A pause records its context
The system SHALL record each new medication-source pause with exactly one source, a reason, an optional note, a start time, and the authorised household member who recorded it. Supported reasons SHALL be `out_of_supply`, `temporarily_not_needed`, `clinician_advice`, `side_effects`, and `other`.

#### Scenario: Pause a formal schedule
- **GIVEN** an active schedule and a user with manage access to its person
- **WHEN** the user pauses it with a supported reason and optional note
- **THEN** the schedule becomes paused and one open pause period records the supplied context and actor

#### Scenario: Pause a direct medication assignment
- **GIVEN** an active routine or as-needed direct assignment and a user with manage access to its person
- **WHEN** the user pauses it with a supported reason
- **THEN** the assignment becomes paused under the same pause-period contract

#### Scenario: Reject missing context on the reason-required workflow
- **GIVEN** an active pausable medication source
- **WHEN** a user submits the reason-required pause workflow without a supported reason
- **THEN** the system rejects the request without pausing the source or creating a pause period

### Requirement: Resume closes rather than removes a pause period
The system SHALL end the source's current pause period when the source resumes and SHALL retain the original reason, note, start, recording actor, end, and resuming actor in history.

#### Scenario: Resume a paused source
- **GIVEN** a source with one open pause period
- **WHEN** an authorised user resumes it
- **THEN** the source becomes active and that period receives an end time and resuming actor
- **AND** the original pause context remains available in history

#### Scenario: Prevent overlapping pause periods
- **GIVEN** a source already has an open pause period
- **WHEN** another pause request is processed
- **THEN** the system returns the existing paused state without creating a second open period

### Requirement: Pause periods control dose expectations
The system SHALL NOT generate expected doses, outstanding tasks, missed-dose escalation, or poor-adherence results inside a known pause period. A resumed source SHALL generate later expectations under its unchanged medication rules.

#### Scenario: Report across a completed pause
- **GIVEN** a reporting range contains active time, a completed pause period, and resumed time for one source
- **WHEN** the system calculates expected doses
- **THEN** it excludes the pause period and includes only applicable active periods

#### Scenario: Show active pause context
- **GIVEN** a source is currently paused
- **WHEN** an authorised user views the medication source
- **THEN** the view identifies it as paused and shows the recorded reason and note when present

### Requirement: Pause writes preserve tenant, audit, and transaction boundaries
The system SHALL authorize pause and resume against the source person, keep pause data inside the active household, record attributable audit evidence, and update the source and pause period atomically. Public failures MUST NOT disclose hidden sources, people, medication names, reasons, or notes.

#### Scenario: Deny a visible source without manage access
- **GIVEN** a user can view a source but lacks manage access to its person
- **WHEN** the user attempts to pause or resume it
- **THEN** the system denies the action and commits no source, pause-period, audit-version, or sync change

#### Scenario: Hide a source from another household
- **GIVEN** a source belongs to a different household
- **WHEN** a user addresses it through a pause operation
- **THEN** the system returns the normal not-found response without disclosing pause context

#### Scenario: Roll back an incomplete pause
- **GIVEN** either the source-state write or pause-period write cannot complete
- **WHEN** the pause transaction fails
- **THEN** neither write nor its audit and sync side effects remain committed

### Requirement: Pause history is portable and API-compatible
The system SHALL expose pause periods through additive product-API, sync, and portable-v2 contracts using portable identifiers. It SHALL advertise support through capabilities and preserve existing `/api/v1` pause requests during the documented deprecation period.

#### Scenario: Use the reason-required API operation
- **GIVEN** a compatible client has discovered pause-period support
- **WHEN** it submits an authorised pause with a supported reason and idempotency key
- **THEN** the API returns the created pause period and matching retries replay one committed result

#### Scenario: Use the deprecated pause operation
- **GIVEN** an older conforming client calls the existing pause operation without context
- **WHEN** the request succeeds during the deprecation period
- **THEN** the source pauses and history explicitly records `reason_not_recorded`

#### Scenario: Round-trip pause history
- **GIVEN** a household has current and completed pause periods
- **WHEN** portable-v2 data is exported and restored
- **THEN** source relationships, reasons, notes, times, actors where portable, and current paused state are preserved without creating duplicate periods

### Requirement: Existing paused sources gain no invented history
The system SHALL preserve sources that are already paused when this capability deploys, but SHALL NOT invent their original pause reason or start time.

#### Scenario: Migrate an existing paused source
- **GIVEN** a source is inactive without a pause-period record before migration
- **WHEN** pause-period support is deployed
- **THEN** it remains paused with one open legacy period marked `reason_not_recorded`
- **AND** its unknown historical start remains explicitly unknown
