## Purpose

Defines stable, explicit outcomes for formal scheduled dose occurrences so deliberate non-administration is distinct from a take or an unexplained miss.

## Requirements

### Requirement: Formal routine schedules expose stable occurrences
The system SHALL identify each expected non-PRN schedule occurrence by its source, applicable date or dose window, and ordinal position. The identity SHALL remain stable when the same occurrence is read through web, API, sync, history, or reporting surfaces.

#### Scenario: Identify timed occurrences
- **GIVEN** a non-PRN schedule expects more than one configured time on an applicable date
- **WHEN** occurrences for that date are requested
- **THEN** each expected dose has a distinct stable identity and its configured time

#### Scenario: Identify an untimed occurrence
- **GIVEN** a non-PRN schedule expects a dose without a configured time
- **WHEN** occurrences for its applicable date are requested
- **THEN** the occurrence has a stable identity without an invented administration time

#### Scenario: Exclude PRN schedules
- **GIVEN** a retained PRN or as-needed schedule
- **WHEN** occurrences are requested
- **THEN** the system generates no expected occurrence to resolve

#### Scenario: Exclude a pause period
- **GIVEN** an otherwise applicable schedule is paused for the complete occurrence window
- **WHEN** occurrences are requested
- **THEN** the system generates no expected occurrence inside that pause period

### Requirement: A due occurrence can be recorded as not taken
The system SHALL allow a user with record access to resolve a due or overdue occurrence as `not_taken` with an optional supported reason and optional note. Supported reasons SHALL be `refused`, `unwell`, `asleep`, `medicine_unavailable`, `clinician_advice`, and `other`.

#### Scenario: Record not taken
- **GIVEN** an unresolved due occurrence and an authorised recorder
- **WHEN** the recorder marks it not taken with optional context
- **THEN** one attributable outcome resolves the occurrence and remains visible in history
- **AND** no `MedicationTake` is created and no stock is reduced

#### Scenario: Reject a future outcome
- **GIVEN** an occurrence is not yet due
- **WHEN** a user attempts to mark it not taken
- **THEN** the system rejects the outcome and leaves the occurrence unresolved

#### Scenario: Backdate a valid overdue occurrence
- **GIVEN** an unresolved past occurrence falls inside the schedule's valid active period
- **WHEN** an authorised API client records it as not taken
- **THEN** the outcome is accepted with the occurrence's original identity

### Requirement: An occurrence resolves exactly once under concurrency
The system SHALL serialize competing outcomes for one occurrence and SHALL commit at most one current resolution with its audit, stock, and sync side effects.

#### Scenario: Competing not-taken submissions converge
- **GIVEN** two authorised requests concurrently submit the same not-taken outcome and idempotency key
- **WHEN** both are processed
- **THEN** both successful responses identify one outcome
- **AND** exactly one resolution and one set of audit and sync changes exists

#### Scenario: Take and not-taken race
- **GIVEN** an unresolved occurrence receives concurrent taken and not-taken requests
- **WHEN** the requests are serialized
- **THEN** one outcome commits and the other receives a stable already-resolved conflict
- **AND** stock changes only when the committed outcome is taken

### Requirement: Corrections preserve clinical history
The system SHALL allow an authorised manager to reopen a not-taken occurrence or replace it with a take while retaining attributable evidence of the former outcome. A persisted `MedicationTake` SHALL remain immutable.

#### Scenario: Reopen not taken
- **GIVEN** an occurrence is currently not taken
- **WHEN** an authorised manager reopens it
- **THEN** the occurrence becomes unresolved and actionable again
- **AND** audit history retains the former reason, note, actor, and resolution time

#### Scenario: Replace not taken with an administration
- **GIVEN** an occurrence is currently not taken and the dose can now be validly recorded
- **WHEN** an authorised recorder takes it through the normal administration workflow
- **THEN** the occurrence becomes taken, one immutable take is created, and stock changes once
- **AND** audit history retains the prior not-taken outcome

#### Scenario: Refuse to reopen a take
- **GIVEN** an occurrence is linked to a persisted medication take
- **WHEN** a user attempts to reopen its outcome
- **THEN** the system rejects the correction and leaves the take unchanged

### Requirement: Outcome consumers distinguish resolution from administration
The system SHALL remove taken and not-taken occurrences from outstanding tasks and missed-dose escalation. Reports SHALL present taken, not-taken, and unexplained missed occurrences separately, and only unexplained misses SHALL feed missed-dose pattern insights.

#### Scenario: Not taken stops escalation
- **GIVEN** a due occurrence has been recorded as not taken
- **WHEN** dashboard and missed-dose eligibility are calculated
- **THEN** the occurrence is not outstanding and no missed-dose notification is sent for it

#### Scenario: Report explicit categories
- **GIVEN** a reporting period contains taken, not-taken, and unresolved overdue occurrences
- **WHEN** adherence history is produced
- **THEN** each category is reported separately without describing not taken as administered

### Requirement: Outcome access is household-scoped and PHI-safe
The system SHALL authorize occurrence reads and writes through person-level access, preserve household isolation, record attributable audit evidence, and return PHI-safe errors.

#### Scenario: Record access can resolve an occurrence
- **GIVEN** a user has record access to the occurrence's person
- **WHEN** the user records a valid not-taken outcome
- **THEN** the operation succeeds for that person and household

#### Scenario: Manage access is required for correction
- **GIVEN** a user has record but not manage access
- **WHEN** the user attempts to reopen a resolved occurrence
- **THEN** the system denies the correction without changing history

#### Scenario: Hide a cross-household occurrence
- **GIVEN** an occurrence belongs to another household
- **WHEN** a client addresses its identity
- **THEN** the system returns the normal not-found response without disclosing clinical context

### Requirement: Scheduled outcomes are portable and additive
The system SHALL advertise scheduled-outcome support, expose bounded occurrence reads and idempotent outcome mutations through additive `/api/v1` operations, include persisted outcomes in sync and portable-v2 data, and retain older valid take requests.

#### Scenario: Discover and resolve through the API
- **GIVEN** a compatible client discovers scheduled-outcome support
- **WHEN** it reads a bounded date range and resolves one returned occurrence
- **THEN** the API uses a stable opaque occurrence identity and returns the resulting current outcome

#### Scenario: Restore outcome history
- **GIVEN** portable-v2 data contains outcomes and linked takes
- **WHEN** it is restored transactionally
- **THEN** occurrence identities, outcomes, reasons, notes, source links, take links, and audit attribution available to the format are preserved
- **AND** restoring history does not replay stock changes

#### Scenario: Preserve a legacy take
- **GIVEN** a take predates explicit occurrence identity
- **WHEN** occurrences and reports are calculated
- **THEN** the take remains valid and may satisfy the earliest compatible occurrence without creating a fabricated persisted link
