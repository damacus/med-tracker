# Spec Delta

## Purpose

Allow households to control whether medication suggestion inputs leave MedTracker for an external AI provider, with consent disabled until explicitly granted.

## ADDED Requirements

### Requirement: Consent defaults to off
The system SHALL treat existing and new households as not opted in unless an authorised manager explicitly enables AI suggestions. Feature flags, plan eligibility, imports and another household's consent MUST NOT imply consent.

#### Scenario: No prior consent
- **GIVEN** an existing or newly created eligible household without explicit consent
- **WHEN** an authenticated member requests AI medication suggestions
- **THEN** the request is refused without any outbound suggestion-related request.

### Requirement: Managers control household consent
The system SHALL allow current household owners and administrators to enable or disable consent through household settings with clear disclosure of external transmission. Other members and foreign-household actors MUST NOT change consent. Changes SHALL retain household isolation, CSRF protection and attributable audit.

#### Scenario: Manager opts in
- **GIVEN** a current household manager and consent disabled
- **WHEN** the manager explicitly enables consent after seeing the disclosure
- **THEN** consent and its actor-attributed audit are persisted atomically for that household only.

#### Scenario: Unauthorised or failed update
- **GIVEN** a non-manager, foreign-household actor, invalid CSRF request or persistence failure
- **WHEN** consent is changed
- **THEN** no consent change is committed and errors disclose no clinical data.

#### Scenario: Repeated settings submission
- **GIVEN** consent already has the requested value
- **WHEN** the same authorised settings update is repeated
- **THEN** the state remains unchanged and no outbound AI request is triggered.

### Requirement: Every outbound suggestion requires current consent
The system SHALL enforce current household consent server-side before outbound suggestion work, alongside existing authority, feature and plan checks. A successful withdrawal SHALL prevent subsequent sends, including later provider calls in an in-flight suggestion. Already transmitted data cannot be recalled.

#### Scenario: Enabled household
- **GIVEN** explicit consent and all existing access and feature requirements satisfied
- **WHEN** a member requests suggestions
- **THEN** the provider can receive only the existing permitted suggestion input and the response follows the existing contract.

#### Scenario: Withdrawal
- **GIVEN** consent was enabled and has been withdrawn successfully
- **WHEN** a new request or later outbound step of an in-flight suggestion attempts to send
- **THEN** no further suggestion data is sent for that household.

#### Scenario: Cross-household or stale authority
- **GIVEN** consent in household A only, or revoked requesting membership
- **WHEN** a caller requests suggestions for household B or with revoked authority
- **THEN** access is refused and the outbound request count remains zero.

#### Scenario: Consent cannot be established
- **GIVEN** missing, invalid or unreadable consent state
- **WHEN** suggestions are requested
- **THEN** the operation fails closed without outbound requests or clinical details in errors or logs.
