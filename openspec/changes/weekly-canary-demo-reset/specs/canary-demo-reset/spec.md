## Purpose

Defines a disposable, production-data-free canary environment that returns to a verified synthetic demo baseline every week without affecting production systems.

## ADDED Requirements

### Requirement: Demo mode is explicit and visible
The application SHALL enable disposable demo behavior only when explicitly
configured for demo mode. Authenticated demo users SHALL see that the environment
contains synthetic data and resets every Sunday at 04:15 Europe/London.

#### Scenario: User visits the demo environment
- **GIVEN** application demo mode is enabled
- **WHEN** an authenticated user views an application page
- **THEN** a persistent, accessible demo notice identifies the environment as disposable
- **AND** the notice states the weekly reset day and time

#### Scenario: Application runs outside demo mode
- **GIVEN** application demo mode is not explicitly enabled
- **WHEN** a user views the application or an operator invokes the demo reset
- **THEN** no demo notice is shown
- **AND** the reset exits unsuccessfully before mutating records or files

### Requirement: Canary uses only synthetic baseline data
The canary environment SHALL initialize from a versioned synthetic demo baseline owned by the application and SHALL NOT initialize from, synchronize with, or retain a production database backup. The same application baseline SHALL be used for clean creation, scheduled reset, manual reset, and environment recovery.

#### Scenario: Clean canary initialization
- **GIVEN** a new canary database with no application records
- **WHEN** the canary baseline is loaded
- **THEN** every account, household, person, medicine, schedule, administration, and related record comes from the committed synthetic dataset
- **AND** the baseline contains no push subscription, native device token, API token, live external credential, or production-derived identifier

#### Scenario: Demo scenarios remain representative
- **GIVEN** the synthetic baseline has loaded successfully
- **WHEN** a tester signs in with a documented demo account
- **THEN** the tester can exercise representative household roles, scheduled medication, as-needed medication, and current-date administration history without using real patient data

### Requirement: Destructive reset is canary-only
The reset operation SHALL require independent explicit confirmation that application demo mode is enabled and that every configured data target belongs to canary. It MUST refuse to mutate data when any safety assertion is absent or names a production target.

#### Scenario: Canary safety assertions pass
- **GIVEN** demo reset is explicitly enabled
- **AND** the configured database and Active Storage targets are the expected canary resources
- **WHEN** an operator invokes the reset
- **THEN** the reset is permitted to proceed

#### Scenario: Production-like target is rejected
- **GIVEN** a configured database, application, storage, or environment target does not identify the expected canary environment
- **WHEN** an operator invokes the reset
- **THEN** the operation exits unsuccessfully before deleting database records or uploaded files
- **AND** the failure output contains target categories and safe identifiers but no credentials, health data, subscription endpoints, or file contents

### Requirement: Reset replaces all mutable demo state
Each successful reset SHALL remove all runtime records and uploaded files created or changed since the baseline, then restore exactly the current committed demo baseline. Schema metadata required to run the deployed application SHALL be preserved.

#### Scenario: User-created data is removed
- **GIVEN** test users have created accounts, medications, administrations, notification subscriptions, device tokens, sessions, jobs, cache entries, audit records, and uploads after the previous reset
- **WHEN** the reset completes successfully
- **THEN** none of those user-created records or uploaded objects remain
- **AND** only records defined by the current demo baseline remain

#### Scenario: Database baseline load fails
- **GIVEN** the reset has begun
- **WHEN** database cleanup or baseline loading fails before commit
- **THEN** the database changes are rolled back
- **AND** the operation exits unsuccessfully without reporting the baseline as verified

#### Scenario: Reset is repeated without intervening use
- **GIVEN** canary already matches the current demo baseline
- **WHEN** the reset runs again
- **THEN** it succeeds and produces the same observable baseline

### Requirement: Weekly reset is externally scheduled
The canary reset SHALL run every Sunday at 04:15 in the Europe/London timezone from an execution boundary that remains available while application job, cache, and session data are being replaced.

#### Scenario: Scheduled reset starts
- **GIVEN** no canary reset is currently active
- **WHEN** the weekly schedule is reached
- **THEN** exactly one reset execution starts outside the application job queue

#### Scenario: Reset has an exclusive mutation window
- **GIVEN** canary web and queue processes can normally create runtime state
- **WHEN** the scheduled reset enters its destructive stages
- **THEN** user mutation traffic and queue writers are quiesced
- **AND** the Rails reset verifies the baseline and empty canary bucket while web remains stopped
- **AND** ordinary demo traffic resumes only after those invariants pass
- **AND** the wrapper verifies the real application health endpoint after web is restored

#### Scenario: Previous reset is still active
- **GIVEN** a canary reset is already running
- **WHEN** another scheduled start is reached
- **THEN** the new execution does not overlap the active reset

### Requirement: Test-user notifications remain functional
Canary SHALL allow test users to register notification destinations and receive notifications generated from synthetic demo data. Those registrations SHALL be treated as disposable canary state.

#### Scenario: Test user registers a device
- **GIVEN** a demo user is signed in to canary
- **WHEN** the user enables notifications on a test device
- **THEN** canary stores the test subscription or device token and can deliver canary reminders to it

#### Scenario: Weekly reset follows notification testing
- **GIVEN** a test user registered a notification destination during the week
- **WHEN** the weekly reset completes
- **THEN** the registration is removed and is not restored as part of the baseline

### Requirement: Canary has no database backup subsystem
The disposable canary environment SHALL NOT create or retain database backups, backup objects, object-store copies, or write-ahead-log archives. Recreating and migrating a blank database before loading the versioned application baseline SHALL be the recovery procedure.

#### Scenario: Canary database is lost or deliberately purged
- **GIVEN** the canary database and its volumes do not exist
- **WHEN** the canary environment is recreated
- **THEN** a blank database is initialized and migrated
- **AND** the deployed application loads and verifies the same committed baseline used by the weekly reset
- **AND** no database backup or WAL archive is required
- **AND** the empty canary upload bucket is not used as a recovery source

#### Scenario: Mutable demo activity is reset
- **GIVEN** canary contained mutable demo activity before a reset
- **WHEN** the reset completes
- **THEN** canary has no backup or archive path from which the removed activity can be restored

### Requirement: Reset completion is verified safely
A reset SHALL be successful only after verifying baseline identity, expected representative records, absence of out-of-baseline delivery registrations, and availability of the canary application. Verification output MUST be safe for operational logs.

#### Scenario: Baseline verification passes
- **GIVEN** cleanup and baseline loading have completed
- **WHEN** post-reset verification runs
- **THEN** it confirms demo mode is enabled, the expected synthetic accounts and medication scenarios, zero baseline notification registrations, and an empty canary upload bucket
- **AND** the external reset wrapper restores the web deployment and confirms a healthy canary application
- **AND** the reset reports success using counts, statuses, and synthetic identifiers only

#### Scenario: Post-reset verification fails
- **GIVEN** cleanup or baseline loading produced an unexpected state
- **WHEN** post-reset verification detects the mismatch
- **THEN** the reset reports failure and identifies the failed invariant without logging health data, credentials, endpoints, device tokens, or uploaded-file contents
