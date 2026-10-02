# Spec Delta

## Purpose

Preserve MedTracker account registration, verification, recovery and closure behaviour when the Rust application becomes the authentication owner.

## ADDED Requirements

### Requirement: Registration preserves invitation and adult-account rules

The system SHALL preserve invite-only registration, invitation-bound email, adult-only self-registration, and atomic account, person, user, household and access-grant creation. Failed registration MUST NOT leave partial access records.

#### Scenario: Valid invited registration
- **GIVEN** an invitation with a fixed email address
- **WHEN** an eligible adult completes registration
- **THEN** the account and authorised membership use that email and the invitation is accepted atomically

#### Scenario: Failed registration transaction
- **GIVEN** a registration whose person or membership creation fails
- **WHEN** the transaction aborts
- **THEN** no partial account access or consumed invitation remains

#### Scenario: Uninvited signup on invite-only instance
- **GIVEN** an instance that requires invitations
- **WHEN** an uninvited visitor requests registration
- **THEN** registration is denied without granting household access

### Requirement: Verification and account recovery preserve one-use protections

The system SHALL preserve account verification, password reset, verified login change and unlock behaviour, including token expiry, resend rules, one-use mutation, password requirements and safe failures. It SHALL enqueue mail only after the corresponding transaction commits. Responses and diagnostics MUST NOT disclose secrets or medical data.

#### Scenario: Password reset token is consumed once
- **GIVEN** a valid unused password reset token
- **WHEN** two requests attempt to reset the password with it
- **THEN** at most one succeeds and the token cannot be reused

#### Scenario: Invalid recovery request
- **GIVEN** an unknown account or invalid or expired recovery token
- **WHEN** a recovery request is submitted
- **THEN** no credential mutation or authenticated session results and the response does not disclose credential material

#### Scenario: Recovery transaction fails
- **GIVEN** a recovery mutation that rolls back
- **WHEN** the transaction fails
- **THEN** no recovery mail is queued and the account remains unchanged

### Requirement: Account closure retains medical history

The system SHALL close the authentication account and revoke applicable credentials while retaining people and medical history according to the Rails closure contract. Closure MUST NOT grant another account access to retained records.

#### Scenario: Close an account with recorded medication history
- **GIVEN** an account linked to a person with medication history
- **WHEN** the account closes successfully
- **THEN** authentication is denied, the person-account link is cleared, and the person and medication history remain
