# Spec Delta

## Purpose

Provide safe, accessible Rust browser access to existing health-history PDF API behaviour without changing domain permissions.

## ADDED Requirements

### Requirement: Valid PDF
The browser SHALL implement the following observable behaviour.

#### Scenario: Valid PDF
- **GIVEN** an eligible actor can manage the selected person
- **WHEN** they download with valid dates and medication takes set to 1 or 0
- **THEN** the existing PDF contains the selected person's matching history and includes or omits takes accordingly, with attachment and no-store headers.

### Requirement: Invalid date range
The browser SHALL implement the following observable behaviour.

#### Scenario: Invalid date range
- **GIVEN** dates are malformed, reversed or exceed 366 elapsed days
- **WHEN** Download is submitted
- **THEN** filters remain visible with associated errors and no successful attachment.

### Requirement: Masked access failure
The browser SHALL implement the following observable behaviour.

#### Scenario: Masked access failure
- **GIVEN** a foreign person identifier, revoked grant or ineligible actor is used
- **WHEN** a download is requested
- **THEN** existing API permissions reject the request without disclosing health data.

### Requirement: Empty states
The browser SHALL implement the following observable behaviour.

#### Scenario: Empty states
- **GIVEN** there are no manageable people or the authorised person has no matching history
- **WHEN** Reports is used
- **THEN** no-person state disables Download, while empty history retains the API's valid empty PDF behaviour.

### Requirement: Failure and audit
The browser SHALL implement the following observable behaviour.

#### Scenario: Failure and audit
- **GIVEN** a report generation failure occurs or a download is repeated
- **WHEN** the API processes the request
- **THEN** no partial PDF is returned on failure and existing transaction/audit semantics apply once per request without an extra preview call.

### Requirement: Binary and session safety
The browser SHALL implement the following observable behaviour.

#### Scenario: Binary and session safety
- **GIVEN** an API session is refreshed, a body is oversized or a JSON error is returned
- **WHEN** the adapter handles the response
- **THEN** cookies are preserved, response bytes are bounded, errors are controlled and existing JSON callers remain unchanged.

### Requirement: Accessible filters
The browser SHALL implement the following observable behaviour.

#### Scenario: Accessible filters
- **GIVEN** any supported locale and desktop or mobile viewport
- **WHEN** the person uses filters and errors by keyboard
- **THEN** labels, focus, date guidance and associated feedback remain usable without untranslated keys.
