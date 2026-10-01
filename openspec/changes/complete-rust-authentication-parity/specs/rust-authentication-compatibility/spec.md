# Spec Delta

## Purpose

Allow existing MedTracker accounts to authenticate through Rust while preserving enrolled credentials, authentication assurance and household access boundaries.

## ADDED Requirements

### Requirement: Existing credentials survive authentication cutover

The system SHALL preserve account IDs, password hashes, passkey credential IDs and user handles, existing authenticator secrets, and unused recovery codes. It MUST NOT enable an imported authentication method before its compatibility and assurance have been verified.

#### Scenario: Existing authenticator app
- **GIVEN** an existing account with an authenticator enrolled through Rails
- **WHEN** the account submits the correct code within the accepted time window
- **THEN** Rust authenticates the same account without requiring re-enrolment

#### Scenario: Existing discoverable passkey
- **GIVEN** an existing account with a Rails-created discoverable passkey
- **WHEN** the browser submits a valid verified assertion for the configured origin
- **THEN** the same credential and existing user handle identify the same account

#### Scenario: Unproved credential format
- **GIVEN** a credential format whose import compatibility is unverified
- **WHEN** login attempts to use that credential
- **THEN** login remains unavailable without bypassing its enrolled factor

### Requirement: OTP and recovery submissions cannot be reused

The system SHALL retain the accepted OTP time window, last-use rule and failure limit, and SHALL consume recovery codes once. Concurrent requests MUST NOT both authenticate using the same accepted OTP or recovery code. Failures MUST reveal no credential material.

#### Scenario: Accepted OTP interval cannot be repeated
- **GIVEN** an OTP successfully used by an account and persisted last-use state
- **WHEN** the same code is submitted again within the reuse-protection interval
- **THEN** authentication is denied

#### Scenario: Recovery-code race
- **GIVEN** one unused recovery code
- **WHEN** two concurrent requests submit that code
- **THEN** at most one authenticates and the code is consumed atomically

#### Scenario: Invalid authenticator code
- **GIVEN** an account with OTP enabled
- **WHEN** invalid codes reach the existing failure limit
- **THEN** further OTP authentication is denied according to the existing policy

### Requirement: Factor completion establishes trusted session assurance

The system SHALL require completion of the enrolled login factors before authorising browser data or native consent. Credential enrolment, removal and recovery-code viewing SHALL retain their password and MFA protections. Removing action-specific fresh-MFA gates MUST NOT remove these login and credential-management protections.

#### Scenario: Password is only the first factor
- **GIVEN** an account with optional MFA enrolled
- **WHEN** its correct password is submitted without completing the required factor
- **THEN** no authenticated browser data or native authorisation code is granted

#### Scenario: Verified passkey login
- **GIVEN** a valid passkey assertion with required user verification
- **WHEN** login completes
- **THEN** a trusted authenticated session can resume the requested native consent flow

#### Scenario: Adding a second factor to a protected account
- **GIVEN** an account with an existing enrolled factor
- **WHEN** another credential is enrolled
- **THEN** existing password and factor-management protections are enforced

### Requirement: Local and generic OIDC login retain account identity

The system SHALL support configured generic OIDC and local login together, validate the provider signature, issuer, audience, expiry, nonce and state, and preserve existing linked account identities and invitation restrictions. Provider assurance MUST be accepted only from validated claims meeting the configured MFA policy.

#### Scenario: Existing linked provider identity
- **GIVEN** an existing account identity and configured provider
- **WHEN** a validated OIDC callback identifies that identity
- **THEN** login uses the existing account ID without creating a duplicate account

#### Scenario: Forged or replayed provider callback
- **GIVEN** a callback with invalid signature, claims, nonce or state
- **WHEN** it is received
- **THEN** login fails without creating an authenticated session

#### Scenario: Provider did not perform MFA
- **GIVEN** an account that requires an enrolled local factor and a valid provider login without acceptable MFA assurance
- **WHEN** sign-in proceeds
- **THEN** local factor completion remains required

### Requirement: Established session and native authorisation policy remains intact

The system SHALL retain signed database-backed revocable browser sessions, CSRF and origin checks, the default 30-day inactivity timeout and no default absolute maximum. Native login SHALL remain account-level with S256 PKCE, consent, one-use codes, refresh rotation and revocation, and current household and person authorisation on every request. Administrative actions MUST NOT introduce separate action-specific fresh-MFA requirements.

#### Scenario: Revoked session
- **GIVEN** a revoked browser session or mobile grant
- **WHEN** it is reused
- **THEN** protected access is denied

#### Scenario: Household switch after login
- **GIVEN** one account-level native credential
- **WHEN** the client requests an unauthorised household or person
- **THEN** current access checks deny the request without data from another household

#### Scenario: Default interactive session lifetime
- **GIVEN** no lifetime overrides and a valid authenticated session
- **WHEN** its last use exceeds 30 days
- **THEN** reauthentication is required without imposing a separate default absolute age limit
