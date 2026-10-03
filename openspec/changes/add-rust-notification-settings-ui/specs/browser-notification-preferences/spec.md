# Spec Delta

## Purpose

Provide safe, accessible Rust browser access to existing notification preference API behaviour without changing domain permissions.

## ADDED Requirements

### Requirement: Saved preferences
The browser SHALL implement the following observable behaviour.

#### Scenario: Saved preferences
- **GIVEN** an authorised person has existing preferences
- **WHEN** they save all five switches, including unchecked values
- **THEN** the five booleans persist on reload and all four reminder times remain unchanged.

#### Scenario: Reachable from dashboard
- **GIVEN** a signed-in person can read their notification preferences
- **WHEN** they use the dashboard navigation on desktop or the mobile menu
- **THEN** My notifications opens the household settings page; masked access does not expose the link.

### Requirement: First-use preferences
The browser SHALL offer the database-backed default switches to an authorised person whose preference row has not yet been created, without writing until they save.

#### Scenario: First-use preferences
- **GIVEN** an authorised person has no preference row
- **WHEN** they open the page and save the default switches
- **THEN** the page offers the stored defaults and creates a preference row through the existing API.

### Requirement: Masked denied state
The browser SHALL implement the following observable behaviour.

#### Scenario: Masked denied state
- **GIVEN** the preference API masks denied access as 404
- **WHEN** the page renders
- **THEN** a neutral unavailable state appears without editable defaults or automatic creation.

### Requirement: View-only preferences
The browser SHALL show existing preferences without editable switches or a save action when the person lacks management permission.

#### Scenario: View-only preferences
- **GIVEN** the person may view but not manage their preference row
- **WHEN** they open the page
- **THEN** the switches show their values as disabled, an explanation appears and no save action is offered.

### Requirement: Invalid authority
The browser SHALL implement the following observable behaviour.

#### Scenario: Invalid authority
- **GIVEN** access is revoked, a foreign household is selected or CSRF is invalid
- **WHEN** a save is submitted
- **THEN** the request is rejected without exposing or changing another person's preferences.

### Requirement: Failed transaction
The browser SHALL implement the following observable behaviour.

#### Scenario: Failed transaction
- **GIVEN** a person submitted switch values
- **WHEN** the API reports a transaction failure
- **THEN** attempted values remain visible with an associated error, no success is shown and stored values are unchanged.

### Requirement: Unchanged save
The browser SHALL implement the following observable behaviour.

#### Scenario: Unchanged save
- **GIVEN** submitted values equal saved values
- **WHEN** the person saves again
- **THEN** existing API no-op semantics apply without duplicate UI audit writes.

### Requirement: Private content and accessibility
The browser SHALL implement the following observable behaviour.

#### Scenario: Private content and accessibility
- **GIVEN** any supported locale and desktop or mobile viewport
- **WHEN** the person operates switches by keyboard
- **THEN** focus and labels remain usable, and private content true is explained as hiding sensitive notification content.
