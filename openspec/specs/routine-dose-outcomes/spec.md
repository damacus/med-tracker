## Purpose

Extends explicit dose outcomes to expected direct routine medication assignments without turning as-needed medication into scheduled work.

## Requirements

### Requirement: Direct routine assignments expose stable occurrences
The system SHALL identify expected occurrences for an active routine `PersonMedication` from its existing dose-cycle window and ordinal position. It SHALL NOT invent a configured administration time.

#### Scenario: Identify a daily routine occurrence
- **GIVEN** an active direct routine assignment expects one daily dose
- **WHEN** current occurrences are requested
- **THEN** one stable occurrence identifies the current daily window

#### Scenario: Identify multiple routine occurrences
- **GIVEN** an active direct routine assignment permits more than one expected dose in its current cycle
- **WHEN** occurrences are requested
- **THEN** each expected position has a distinct stable identity

#### Scenario: Preserve existing cycle rules
- **GIVEN** a routine assignment uses a daily, weekly, or monthly dose cycle
- **WHEN** occurrences are derived
- **THEN** the system uses the existing cycle boundary and dose limit without changing their meaning

### Requirement: As-needed assignments never generate expected outcomes
The system SHALL NOT create expected occurrences, not-taken actions, missed-dose escalation, or adherence penalties for an as-needed direct assignment.

#### Scenario: View an as-needed assignment
- **GIVEN** an active direct assignment has `as_needed` administration kind
- **WHEN** dashboard, reminder, report, API, or sync occurrence data is produced
- **THEN** no expected occurrence exists for that assignment

### Requirement: Direct routine outcomes reuse the scheduled-outcome contract
The system SHALL apply the same due-or-overdue validation, reasons, stock behaviour, correction rules, authorization, household isolation, audit evidence, dashboard resolution, notification suppression, and reporting categories to a direct routine occurrence.

#### Scenario: Mark a routine assignment not taken
- **GIVEN** an unresolved direct routine occurrence and an authorised recorder
- **WHEN** the recorder marks it not taken with optional context
- **THEN** it resolves without creating a take or changing stock
- **AND** it is removed from outstanding work and missed-dose escalation

#### Scenario: Take after a not-taken correction
- **GIVEN** a direct routine occurrence was not taken and an authorised manager has reopened it
- **WHEN** the dose is recorded through the normal administration workflow
- **THEN** one immutable take resolves the occurrence and stock changes once

#### Scenario: Exclude a direct-assignment pause period
- **GIVEN** a direct routine assignment is paused for an occurrence window
- **WHEN** occurrences are calculated
- **THEN** no expected occurrence is generated inside that pause period

### Requirement: Direct routine outcomes have interface parity
The system SHALL expose direct routine occurrences and persisted outcomes through the same capability-advertised web, API, sync, portable-v2, audit, history, report, and generated-client contracts used for formal scheduled outcomes.

#### Scenario: Resolve through an external client
- **GIVEN** a compatible authorised client reads a direct routine occurrence
- **WHEN** it submits an idempotent not-taken mutation
- **THEN** the response, sync change, and portable record use the same outcome shape and source discriminator as formal scheduled outcomes

#### Scenario: Hide a direct assignment from another household
- **GIVEN** a direct routine assignment belongs to another household
- **WHEN** a client requests or mutates its occurrence
- **THEN** the system returns the normal PHI-safe not-found response and commits no changes
