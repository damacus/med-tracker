## Purpose

Defines an account-gated medication launcher that preserves the canonical assignment workflow while safely using optional person and medication context.

## Requirements

### Requirement: Account-gated context-aware launcher

The system SHALL provide a per-account medication launcher experiment with `current` and `context_aware` variants, and SHALL use `current` for absent or invalid preferences.

#### Scenario: Experiment is off

- **GIVEN** an account uses the `current` launcher variant
- **WHEN** the account launches Add Medication from a person
- **THEN** the system renders the existing person medication landing page

#### Scenario: Add Medication is selected for a person

- **GIVEN** an account uses the `context_aware` launcher variant
- **WHEN** the account selects Add Medication for a person it may manage
- **THEN** the system opens the canonical medication-assignment workflow with that person selected
- **AND** the account can use Back to choose someone else

#### Scenario: Global Add Medication is selected

- **GIVEN** an account uses the `context_aware` launcher variant
- **WHEN** the account selects global Add Medication without person context
- **THEN** the system asks who the medication is for

### Requirement: Launch-context contract

The context-aware launcher SHALL accept optional `person_id`, `medication_id`, `intent`, and `return_to` context. Blank intent and `assign_medication` SHALL select the canonical medication-assignment intent, and unsupported intent SHALL fall back to the current person-selection step.

#### Scenario: Authorized person and medication are known

- **GIVEN** the launcher receives an authorized person and an allowed household medication
- **WHEN** the account opens the context-aware launcher
- **THEN** the system opens the canonical assignment workflow with the person visible and the medication preselected
- **AND** the person is changeable through the workflow back action and the medication remains changeable in the form

#### Scenario: Only medication is known

- **GIVEN** the launcher receives an allowed household medication without a person
- **WHEN** the account opens the context-aware launcher
- **THEN** the system asks for a person once and carries the medication into the canonical assignment workflow

#### Scenario: No context is known

- **GIVEN** the launcher receives no person or medication context
- **WHEN** the account opens the context-aware launcher
- **THEN** the system starts at the existing person-selection step

#### Scenario: Return destination is local

- **GIVEN** the launcher receives a local return destination
- **WHEN** the account opens and cancels the canonical assignment workflow
- **THEN** the system returns to that destination

#### Scenario: Return destination is external

- **GIVEN** the launcher receives an external return destination
- **WHEN** the account opens the context-aware launcher
- **THEN** the system discards the external destination

### Requirement: Safe context degradation

The launcher SHALL apply existing household and person authorization before using context, SHALL NOT disclose rejected records, and SHALL delegate medication and assignment validation to the canonical workflow.

#### Scenario: Person context is stale or unauthorized

- **GIVEN** the launcher receives a missing, cross-household, or unauthorized person identifier
- **WHEN** the account opens the context-aware launcher
- **THEN** the system renders the authorized person-selection step without identifying the rejected person

#### Scenario: Medication context is stale or unauthorized

- **GIVEN** the launcher receives a missing or cross-household medication identifier with an authorized person
- **WHEN** the account opens the context-aware launcher
- **THEN** the canonical assignment form leaves medication unselected without identifying the rejected medication

#### Scenario: Household has no eligible people

- **GIVEN** the account has no people for whom it may add medication
- **WHEN** the account opens the context-aware launcher
- **THEN** the system renders the person-selection step without an exception or records from another household

#### Scenario: Medication is already assigned

- **GIVEN** the selected medication already has a current direct assignment for the person
- **WHEN** the account submits the canonical assignment workflow
- **THEN** the system preserves the existing assignment and shows the existing validation error

### Requirement: Existing navigation outcomes

The context-aware launcher SHALL preserve the canonical workflow's established cancellation and successful-completion destinations.

#### Scenario: Person-page launch is cancelled

- **GIVEN** the account starts the context-aware launcher from a person page
- **WHEN** the account cancels the canonical workflow
- **THEN** the system returns to that person page

#### Scenario: Assignment completes

- **GIVEN** the account submits a valid canonical medication assignment
- **WHEN** creation succeeds
- **THEN** the system returns to the selected person's page using the existing success behavior
