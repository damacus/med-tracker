## Purpose

Let authenticated native users maintain their profile and complete invitations without weakening identity or household access.

## ADDED Requirements

### Requirement: Update the current profile
The API SHALL permit only existing non-security profile preferences and date of birth, with validation and atomic multi-record updates; email credentials and roles SHALL not be writable.

#### Scenario: Save preferences
- **GIVEN** an authenticated current profile
- **WHEN** valid time zone, shortcuts or profile fields are submitted
- **THEN** validated values are returned for the current identity

#### Scenario: Roll back invalid changes
- **GIVEN** a mixed profile update contains an invalid preference
- **WHEN** the request is submitted
- **THEN** no partial profile change persists

#### Scenario: Reject privilege fields
- **GIVEN** an authenticated client
- **WHEN** email credentials or role changes are submitted through profile
- **THEN** those fields cannot alter identity or authority

### Requirement: Protect avatars
The API SHALL support authenticated current-person avatar upload, read and removal with existing image validation and fresh read authorization.

#### Scenario: Upload a valid image
- **GIVEN** a current person and permitted image
- **WHEN** the client uploads it
- **THEN** a protected attachment is available to authorized reads

#### Scenario: Reject invalid or unauthorized access
- **GIVEN** an invalid image or revoked reader
- **WHEN** an upload or read is attempted
- **THEN** no invalid attachment is committed and no protected image is disclosed

### Requirement: Accept and resend invitations online
The API SHALL require a verified matching authenticated identity for invitation acceptance, enforce pending/expiry/revocation state transactionally, and require fresh privileged authorization for resend.

#### Scenario: Accept once
- **GIVEN** a pending invitation and verified matching account
- **WHEN** acceptance is retried
- **THEN** one membership and intended grants exist without duplicate authority

#### Scenario: Refuse unusable invitations
- **GIVEN** an expired, revoked or mismatched invitation
- **WHEN** acceptance is requested
- **THEN** no membership or grant is created and the failure is non-disclosing

#### Scenario: Resend with current authority
- **GIVEN** an authorized administrator with fresh privileged authentication
- **WHEN** resend is requested
- **THEN** the existing workflow issues a replacement invitation and the old token no longer works
