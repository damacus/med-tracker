## Purpose

Preserve offline medication records and provide conservative, understandable dose controls while disconnected.

## ADDED Requirements

### Requirement: Queued history survives recoverable failures
The browser SHALL preserve the original queued dose and UUID after transient HTTP, transport, malformed-response or authentication failures, and SHALL explain how to retry or sign in. Permanent rejections SHALL be retained atomically for review.

#### Scenario: Temporary failure and retry
- **GIVEN** a queued dose
- **WHEN** syncing encounters a timeout, 429, 503 or invalid response
- **THEN** the dose remains pending with its original UUID and a retry action is available
- **AND** a later successful retry removes it from pending storage

#### Scenario: Authentication required
- **GIVEN** a queued dose
- **WHEN** syncing requires authentication
- **THEN** the queue remains intact and the UI explains that sign-in is required

#### Scenario: Failure storage aborts
- **GIVEN** a permanently rejected queued dose
- **WHEN** storing the rejection fails
- **THEN** the original queued dose remains available for retry

#### Scenario: Tenant isolation
- **GIVEN** queued doses for two household memberships
- **WHEN** one membership synchronises
- **THEN** the other membership's doses remain untouched

#### Scenario: Only permanent rejections remain
- **GIVEN** no pending sync failure and a retained rejected dose
- **WHEN** the recovery panel is displayed
- **THEN** the rejection is visible without an ineffective retry button

### Requirement: Stock selection includes pending consumption
The browser SHALL choose available stock after accounting for pending doses and SHALL prevent overlapping repeated submissions of a single action.

#### Scenario: First stock location is locally exhausted
- **GIVEN** two matching stock locations and pending doses consuming the first
- **WHEN** a dose from another eligible source is queued
- **THEN** it uses the remaining stock location

#### Scenario: Concurrent reservations
- **GIVEN** separate browser documents sharing the same pending-dose store
- **WHEN** both attempt to reserve the last available unit
- **THEN** stock selection and insertion are serialised and only one reservation succeeds

#### Scenario: Different medicines are clicked rapidly
- **WHEN** two different eligible medicine buttons are clicked while storage is pending
- **THEN** both actions are queued and a duplicate click on the same button is ignored

#### Scenario: A dose consumes multiple stock units
- **GIVEN** a countable-unit or volume dose greater than the remaining stock
- **WHEN** its offline action is rendered
- **THEN** the action is disabled
- **AND** pending doses deduct their full quantity from displayed supply

### Requirement: Offline controls reflect cached eligibility
The browser SHALL disable dose actions for sources that the server assessed as unavailable or unauthorised, for outdated eligibility, and when pending doses make the cached decision uncertain. It SHALL display a reason and SHALL preserve server validation on sync.

#### Scenario: Restricted source
- **GIVEN** a paused, inactive, out-of-date, cooldown-restricted or view-only source
- **WHEN** the offline snapshot is rendered
- **THEN** the dose action is disabled with an explanation

#### Scenario: Pending overlapping medicine
- **GIVEN** a pending dose for a person and medicine
- **WHEN** another matching source is rendered
- **THEN** another dose requires syncing first

#### Scenario: Old snapshot
- **GIVEN** cached eligibility is absent or from a previous date
- **WHEN** the offline screen is displayed
- **THEN** it asks for a refreshed snapshot before another dose can be queued

#### Scenario: A larger care plan
- **WHEN** schedules and direct medicines are added for existing people
- **THEN** eligibility and forecast reads remain batched rather than querying for each source
- **AND** delegated stock access, overlapping prescriptions and timing rules retain their existing decisions
