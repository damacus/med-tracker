## Purpose

Protects MedTracker credentials and health information by making secure authentication, storage, diagnostics, backup, and live-test boundaries observable Android release properties.

## ADDED Requirements

### Requirement: Android authentication uses Rodauth browser PKCE
As superseded by `unify-mobile-login-with-rodauth`, every Android build SHALL use the selected MedTracker instance's browser authorization-code flow with S256 PKCE. The application SHALL offer an editable instance URL and labelled canary/demo presets and SHALL NOT present a native password form.

#### Scenario: Release sign-in
- **WHEN** a user signs in to a release build
- **THEN** the application validates discovery for the selected instance and starts Rodauth browser PKCE without native password fields

#### Scenario: Staging sign-in
- **WHEN** a tester uses the staging build
- **THEN** the application uses the same instance selection and Rodauth browser flow with its registered staging callback

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
