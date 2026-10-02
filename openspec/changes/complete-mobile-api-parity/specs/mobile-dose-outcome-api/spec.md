## Purpose

Let native clients resolve expected dose occurrences while preserving immutable administration history.

## ADDED Requirements

### Requirement: Bounded occurrence discovery
The API SHALL expose opaque, stable occurrence identities for authorized people over a required date range of at most 31 days, with timed or untimed state and concrete source type; PRN sources SHALL be absent.

#### Scenario: Read without side effects
- **GIVEN** an accessible routine schedule and bounded date range
- **WHEN** the client lists occurrences twice
- **THEN** the same identities are returned and no occurrence rows are created

#### Scenario: Hide inaccessible sources
- **GIVEN** another household or a person without view access
- **WHEN** the client requests their occurrences
- **THEN** the response exposes no clinical data

#### Scenario: Reject unbounded history
- **GIVEN** a missing, reversed or over-31-day range
- **WHEN** the client lists occurrences
- **THEN** a stable validation error is returned

### Requirement: Audited resolution and correction
The API SHALL implement the existing scheduled and routine outcome semantics: due not-taken with optional reason/note, manage-authorized reopen, and canonical take linkage. Persisted takes SHALL remain immutable.

#### Scenario: Resolve without administration
- **GIVEN** an unresolved due occurrence and record access
- **WHEN** not-taken is submitted with an idempotency key
- **THEN** one audited outcome is saved without a take or stock change and reminders stop

#### Scenario: Serialize competition
- **GIVEN** competing take and not-taken submissions
- **WHEN** the requests resolve the same occurrence
- **THEN** one resolution commits and the other receives a stable conflict

#### Scenario: Preserve audit on reopen
- **GIVEN** a not-taken outcome and manage access
- **WHEN** the client reopens with the current ETag
- **THEN** it becomes open and prior reason, note and actor remain auditable

#### Scenario: Reject take correction
- **GIVEN** an occurrence linked to a take
- **WHEN** the client requests reopen
- **THEN** the request fails without changing the take

#### Scenario: Protect replacement from competing decisions
- **GIVEN** an occurrence was recorded as not taken
- **WHEN** a client attempts to replace it with a take without its current ETag
- **THEN** the API rejects the missing or stale precondition without changing history or stock
- **AND** an explicit replacement with the current ETag uses the canonical administration transaction

#### Scenario: Retain legacy clients
- **GIVEN** a valid take request without an occurrence key
- **WHEN** the client records the dose
- **THEN** existing response and stock semantics remain valid
