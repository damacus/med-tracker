## ADDED Requirements

### Requirement: Queued takes can identify a formal scheduled occurrence
The system SHALL accept an optional stable occurrence identity on a queued medication-take create. When present, the occurrence and take SHALL resolve atomically through the normal medication-administration boundary without changing the behaviour of an existing valid request that omits it.

#### Scenario: Resolve an occurrence with a queued take
- **GIVEN** an authenticated client can record for the person and has a valid unresolved formal scheduled occurrence
- **WHEN** it creates a medication take with that occurrence identity and a new `client_uuid`
- **THEN** one immutable take is created and the occurrence becomes taken in the same transaction
- **AND** stock, audit, idempotency, and sync side effects occur once

#### Scenario: Reject a mismatched occurrence
- **GIVEN** the occurrence identity does not belong to the submitted medication source or dose window
- **WHEN** the client submits the queued take
- **THEN** the whole batch is rejected with a stable PHI-safe validation error
- **AND** no take, occurrence, stock, audit, or sync side effect remains

#### Scenario: Preserve an older queued client
- **GIVEN** a conforming client does not support explicit occurrences
- **WHEN** it submits the existing valid medication-take create shape
- **THEN** the request retains its documented behaviour and idempotency guarantees
