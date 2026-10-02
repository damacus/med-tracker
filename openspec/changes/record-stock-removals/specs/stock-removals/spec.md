## Purpose

Let household inventory managers explain stock that leaves inventory without recording a medication administration.

## ADDED Requirements

### Requirement: Record a stock removal

The system SHALL subtract a positive quantity from explicitly selected tracked stock, with a required reason and optional note, without changing administration history or scheduled-dose state.

#### Scenario: Dropped unit
- **GIVEN** a medicine with 20 tracked units
- **WHEN** an authorised manager removes 1 unit with reason Dropped
- **THEN** 19 units remain and no medication take is created
- **AND** audit evidence preserves quantity, reason, note, actor, time and before/after stock

#### Scenario: Dosage-specific stock
- **GIVEN** stock is tracked on dosage options
- **WHEN** a manager selects one option and removes stock
- **THEN** only that option is reduced and the medicine total is synchronised

### Requirement: Reject unsafe removals atomically

The system SHALL reject invalid quantities, untracked sources, insufficient stock, invalid reasons and notes exceeding 1,000 characters without partial writes.

#### Scenario: Invalid quantity
- **GIVEN** tracked stock
- **WHEN** zero, negative, non-finite, non-numeric, over-precision or excessive quantity is submitted
- **THEN** stock and removal history remain unchanged and an actionable error is displayed

#### Scenario: Persistence failure
- **GIVEN** a valid removal
- **WHEN** audit persistence fails
- **THEN** all stock changes roll back and success is not reported

### Requirement: Isolate and authorise stock changes

The system SHALL apply existing inventory-management permissions and household boundaries to the form, stock selection, mutation and removal history.

#### Scenario: Forbidden access
- **GIVEN** an account without inventory update authority or a medicine in another household
- **WHEN** it requests the form or submits a removal
- **THEN** no stock changes and no private removal details are returned

### Requirement: Safely retry and serialize removals

The system SHALL prevent a submitted removal from being applied twice and check stock after obtaining exclusive access.

#### Scenario: Duplicate submission
- **GIVEN** a successful submission
- **WHEN** the same submission identifier and payload are retried
- **THEN** success is returned with no second stock reduction or removal event

#### Scenario: Changed retry
- **GIVEN** a successful submission identifier
- **WHEN** that identifier is reused with different removal details
- **THEN** the request fails without changing stock

#### Scenario: Competing stock changes
- **GIVEN** two requests whose combined quantities exceed available stock
- **WHEN** both attempt removal
- **THEN** stock never becomes negative and only valid reductions are committed

### Requirement: Accessible and private web workflow

The system SHALL offer labelled quantity, source, reason and note controls, retain invalid input, and display recent authorised removal history.

#### Scenario: Complete the form
- **GIVEN** a desktop or mobile user on a tracked medicine page
- **WHEN** they open Remove stock and submit valid details
- **THEN** the updated quantity and confirmation are visible
- **AND** the workflow is keyboard operable

#### Scenario: Private validation failure
- **GIVEN** a form containing a private note
- **WHEN** validation fails
- **THEN** the form retains the input with an associated error
- **AND** request diagnostics do not expose the submitted note
