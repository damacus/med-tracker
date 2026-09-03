## Purpose

Protects MedTracker credentials and health information by making secure authentication, storage, diagnostics, backup, and live-test boundaries observable Android release properties.

## ADDED Requirements

### Requirement: Android release authentication uses OIDC PKCE
The Android release application SHALL use OIDC authorization-code authentication with PKCE and SHALL NOT expose password authentication or a user-editable server URL. Password authentication and server overrides MAY exist only in explicitly labelled debug or staging builds.

#### Scenario: Release sign-in
- **WHEN** a user signs in to a release build
- **THEN** the application starts the configured OIDC PKCE flow and offers no password fields or server override

#### Scenario: Staging sign-in
- **WHEN** a tester uses the staging build
- **THEN** the application may present password authentication and a staging-labelled server configuration

### Requirement: Android credentials use protected no-backup storage
The Android application SHALL protect access and refresh credentials with Android Keystore-backed encryption, store encrypted material under no-backup storage, and disable application backup.

#### Scenario: Session persisted
- **WHEN** the phone application persists an authenticated session
- **THEN** no token or password is written to ordinary preferences, logs, or backup-eligible storage

### Requirement: Android network diagnostics are PHI-safe
The release application SHALL disable HTTP logging. Debug and staging builds MAY enable BASIC logging only through an explicit opt-in and SHALL redact authorization and session headers and never log bodies.

#### Scenario: Release request
- **WHEN** a release build performs an authenticated API request
- **THEN** the request body, response body, credentials, and session headers are absent from logs

### Requirement: Live canary tests are opt-in
Tests that access a live MedTracker environment SHALL run only through a dedicated opt-in integration task, SHALL receive credentials through the environment, and SHALL be excluded from ordinary tests and pull-request CI.

#### Scenario: Pull-request CI
- **WHEN** Android pull-request verification runs
- **THEN** no live MedTracker environment is contacted and no literal demo credential is required
